import React, { useContext, useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { TelemetryContext } from '../hooks/TelemetryContext.js';
import { StatusPill } from './States.jsx';
import Icon, { GhostMark } from './Icon.jsx';
import { SCREENS, hrefFor, nextNavIndex } from '../lib/routes.js';
import { CONNECTION_TEXT } from '../lib/connection.js';
import { THEME_OPTIONS, getStorage } from '../lib/theme.js';
import { fetchTunnelInfo } from '../lib/labApi.js';

const KIND_PILL = {
  live: ['ok', 'ok'],
  loading: ['info', 'loading'],
  stale: ['warn', 'clock'],
  degraded: ['warn', 'warn'],
  disconnected: ['critical', 'disconnected'],
  error: ['critical', 'error'],
};

/** The list the Menu button controls. One app, one nav: a fixed id keeps the header button and the nav in step. */
export const NAV_LIST_ID = 'app-nav-list';
export const MENU_BUTTON_ID = 'app-menu-button';

export const PersonaContext = React.createContext({ persona: 'executive', setPersona: () => {} });
export function usePersona() {
  return useContext(PersonaContext);
}

/**
 * Persona control: Executive Dyno Rig vs SRE Precision Cockpit.
 * Allows instant toggling between 5-second business/architectural clarity and microsecond kernel telemetry.
 */
export function PersonaToggle() {
  const { persona = 'executive', setPersona = () => {} } = usePersona();
  return (
    <div className="seg persona-toggle" role="group" aria-label="Control View">
      <button
        type="button"
        className="seg__btn"
        aria-pressed={persona === 'buyer'}
        data-persona-option="buyer"
        onClick={() => setPersona('buyer')}
        title="Shopper (Buyer): Sub-millisecond retail experience & live latency breakdown"
      >
        <Icon name="buyer" />
        <span className="persona-toggle__label">Shopper</span>
      </button>
      <button
        type="button"
        className="seg__btn"
        aria-pressed={persona === 'seller'}
        data-persona-option="seller"
        onClick={() => setPersona('seller')}
        title="Merchant (Seller): Flash sales, stock management & 0.18ms invalidation pulse"
      >
        <Icon name="seller" />
        <span className="persona-toggle__label">Merchant</span>
      </button>
      <button
        type="button"
        className="seg__btn"
        aria-pressed={persona === 'executive'}
        data-persona-option="executive"
        onClick={() => setPersona('executive')}
        title="Executive Dyno: Interactive Crash-Test Rig & 100k req/s database shielding"
      >
        <Icon name="dyno" />
        <span className="persona-toggle__label">Dyno Rig</span>
      </button>
      <button
        type="button"
        className="seg__btn"
        aria-pressed={persona === 'sre'}
        data-persona-option="sre"
        onClick={() => setPersona('sre')}
        title="SRE Cockpit: 4-Stage Kernel Pipeline, Rotary SIEVE Drum & Microsecond Traces"
      >
        <Icon name="cockpit" />
        <span className="persona-toggle__label">SRE Cockpit</span>
      </button>
    </div>
  );
}

/**
 * Cluster fleet mTLS quick status strip (Nodes 1, 2, 3).
 * Visually confirms active 3-node mesh health directly in the top header.
 */
export function FleetMeshIndicator() {
  const nodes = [
    { id: 'node-1', port: 18185, label: 'N1' },
    { id: 'node-2', port: 18186, label: 'N2' },
    { id: 'node-3', port: 18187, label: 'N3' },
  ];
  return (
    <div className="fleet-mesh-strip" aria-label="3-Node mTLS Cluster Mesh">
      {nodes.map((n, i) => (
        <span
          key={n.id}
          className="fleet-mesh-pill"
          data-node-index={i + 1}
          title={`Cluster Member ${n.label} (127.0.0.1:${n.port}) mTLS Mesh Active`}
        >
          <span className="fleet-mesh-pill__dot" />
          <span className="fleet-mesh-pill__name">{n.label}</span>
        </span>
      ))}
    </div>
  );
}

/**
 * 3 Standalone Interfaces & Cloudflare Edge Tunnel Launcher Control.
 * Gives users immediate 1-click access to open the 3 interfaces in separate windows or on mobile phones.
 */
export function MultiInterfaceLauncher() {
  const [open, setOpen] = useState(false);
  const [tunnelData, setTunnelData] = useState(null);
  const [copied, setCopied] = useState(false);
  const qrRef = useRef(null);

  useEffect(() => {
    fetchTunnelInfo().then((res) => {
      if (res && res.ok && res.data) {
        setTunnelData(res.data);
      }
    });
  }, [open]);

  const tunnelUrl = tunnelData?.tunnelUrl || null;
  const isOnline = Boolean(tunnelUrl);
  const lanHost = tunnelData?.lanIp
    || (typeof window !== 'undefined' && window.location && window.location.hostname !== 'localhost' && window.location.hostname !== '127.0.0.1'
      ? window.location.hostname
      : '10.110.16.141');
  const activeStoreUrl = tunnelUrl ? `${tunnelUrl}/store` : `http://${lanHost}:3201/store`;
  const activeCrowdUrl = tunnelUrl ? `${tunnelUrl}/crowd` : `http://${lanHost}:3201/crowd`;
  const activeSellerUrl = tunnelUrl ? `${tunnelUrl}/seller` : `http://${lanHost}:3201/seller`;
  const qrApiUrl = `https://api.qrserver.com/v1/create-qr-code/?size=160x160&margin=6&data=${encodeURIComponent(activeStoreUrl)}`;

  // Render offline/standalone QR code using local qrcode.min.js
  useEffect(() => {
    if (!open || !qrRef.current) return;
    const renderQr = () => {
      if (window.QRCode && qrRef.current) {
        qrRef.current.innerHTML = '';
        new window.QRCode(qrRef.current, {
          text: activeStoreUrl,
          width: 150,
          height: 150,
          colorDark: 'rgb(9, 13, 18)',
          colorLight: 'rgb(255, 255, 255)',
          correctLevel: window.QRCode.CorrectLevel ? window.QRCode.CorrectLevel.M : 0,
        });
      }
    };
    if (window.QRCode) {
      renderQr();
    } else {
      const script = document.createElement('script');
      script.src = '/qrcode.min.js';
      script.onload = renderQr;
      document.body.appendChild(script);
    }
  }, [open, activeStoreUrl]);

  const copyUrl = (url) => {
    if (typeof navigator !== 'undefined' && navigator.clipboard) {
      navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <>
      <button
        type="button"
        className="app-header__tunnel-btn"
        onClick={() => setOpen(true)}
        title="Open 3 Standalone Interfaces & Cloudflare Edge Tunnel"
        aria-label="3 Standalone Interfaces and Cloudflare Tunnel"
      >
        <Icon name="tunnel" />
        <span className="app-header__tunnel-btn-text">📱 Mobile QR & Interfaces</span>
        <span className={`tunnel-indicator-dot ${isOnline ? 'online' : 'standby'}`} />
      </button>

      {open && (
        <div className="tunnel-modal-overlay" role="dialog" aria-modal="true" aria-label="Mobile QR Code and Standalone Interfaces">
          <div className="tunnel-modal-dialog">
            <div className="tunnel-modal-header">
              <div className="tunnel-modal-title-row">
                <Icon name="tunnel" />
                <h2 className="tunnel-modal-title">📱 Connect Other Devices & QR Code</h2>
              </div>
              <button
                type="button"
                className="probe-btn"
                onClick={() => setOpen(false)}
                aria-label="Close dialog"
              >
                ✕ Close
              </button>
            </div>

            <p className="tunnel-modal-subtitle">
              Scan the QR code below with any smartphone or external laptop on the same Wi-Fi (or cellular via Cloudflare Tunnel) to shop live, test flash crowds, and inspect zero-stale cache invalidation.
            </p>

            <div className={`tunnel-status-strip ${isOnline ? 'tunnel-status-strip--active' : 'tunnel-status-strip--standby'}`}>
                    <div className="tunnel-status-meta">
                      <span className="tunnel-badge">{isOnline ? '● CLOUDFLARE EDGE ACTIVE' : '● WIRELESS LAN ACTIVE'}</span>
                      <span className="tunnel-url-text">
                        {activeStoreUrl}
                      </span>
                    </div>
                    <button
                      type="button"
                      className="probe-btn"
                      onClick={() => copyUrl(activeStoreUrl)}
                    >
                      {copied ? '✓ Copied' : 'Copy Mobile Store URL'}
                    </button>
                  </div>

                  {/* Functional Live QR Code */}
                  <div className="tunnel-qr-section" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', margin: '16px 0', padding: '16px', background: 'var(--surface-elevated)', borderRadius: '10px', border: '1px solid var(--line)' }}>
                    <span className="tunnel-card-badge" style={{ marginBottom: '8px' }}>📷 Point Phone Camera to Connect</span>
                    <div
                      ref={qrRef}
                      style={{ background: 'var(--surface-raised)', padding: '8px', borderRadius: '10px', display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 4px 12px rgba(0,0,0,0.3)', minWidth: '150px', minHeight: '150px' }}
                    >
                      <img
                        src={qrApiUrl}
                        alt="Scan QR code with phone camera to open PhantomStore"
                        width="150"
                        height="150"
                        style={{ display: 'block', borderRadius: '4px' }}
                      />
                    </div>
                    <a
                      href={activeStoreUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      style={{ marginTop: '10px', fontFamily: 'var(--font-mono)', fontSize: '13px', color: 'var(--accent)', textDecoration: 'none', fontWeight: 600 }}
                    >
                      {activeStoreUrl}
                    </a>
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px' }}>
                      (Works from iPhone, Android or any laptop on this network)
                    </span>
                  </div>

                  <div className="tunnel-interfaces-grid">
                    {/* Interface 1: Standalone Storefront */}
                    <div className="tunnel-card">
                      <div className="tunnel-card-header">
                        <span className="tunnel-card-badge">🛍️ Consumer View</span>
                        <span className="product-tag">Sub-ms Cache</span>
                      </div>
                      <h3 className="tunnel-card-title">1. Mobile Storefront</h3>
                      <p className="tunnel-card-desc">
                        Dedicated retail shopping experience with 12 precision hardware items, real photography, live PDP, cart drawer, and 1-click buy.
                      </p>
                      <div className="tunnel-card-actions">
                        <a
                          href={activeStoreUrl}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="product-action-btn"
                        >
                          <span>↗ Open Storefront in New Window</span>
                        </a>
                      </div>
                    </div>

                    {/* Interface 2: Standalone Seller Portal */}
                    <div className="tunnel-card">
                      <div className="tunnel-card-header">
                        <span className="tunnel-card-badge">🏪 Merchant View</span>
                        <span className="product-tag">0.18ms Invalidation</span>
                      </div>
                      <h3 className="tunnel-card-title">2. Standalone Seller Portal</h3>
                      <p className="tunnel-card-desc">
                        Merchant operations for live inventory pricing, instant flash sales (-40%), restock pulses, and real-time incoming orders queue.
                      </p>
                      <div className="tunnel-card-actions">
                        <a
                          href={activeSellerUrl}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="product-action-btn"
                        >
                          <span>↗ Open Merchant Portal in New Window</span>
                        </a>
                      </div>
                    </div>

                    {/* Interface 3: Standalone Crowd Hub */}
                    <div className="tunnel-card">
                      <div className="tunnel-card-header">
                        <span className="tunnel-card-badge">👥 Scale & Load</span>
                        <span className="product-tag">Crowd Engine</span>
                      </div>
                      <h3 className="tunnel-card-title">3. Multi-Device Crowd Hub</h3>
                      <p className="tunnel-card-desc">
                        Distributed traffic generator tracking connected phones and laptops, firing coordinated bursts and PIE sneaker-drop surges.
                      </p>
                      <div className="tunnel-card-actions">
                        <a
                          href={activeCrowdUrl}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="product-action-btn"
                        >
                          <span>↗ Open Crowd Hub in New Window</span>
                        </a>
                      </div>
                    </div>

                    {/* Interface 4: SRE Precision Cockpit */}
                    <div className="tunnel-card">
                      <div className="tunnel-card-header">
                        <span className="tunnel-card-badge">🎛️ Kernel Telemetry</span>
                        <span className="product-tag">SIEVE & Prometheus</span>
                      </div>
                      <h3 className="tunnel-card-title">4. SRE Precision Cockpit</h3>
                      <p className="tunnel-card-desc">
                        4-stage kernel pipeline, animated Rotary SIEVE Eviction Drum, microsecond request traces, and raw OpenMetrics exposition.
                      </p>
                      <div className="tunnel-card-actions">
                        <a
                          href="/#persona=sre"
                          target="_blank"
                          rel="noopener noreferrer"
                          className="product-action-btn"
                        >
                          <span>↗ Open Cockpit in New Window</span>
                        </a>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </>
  );
}

/**
 * Portals the persona toggle and cluster fleet mesh into the header slots
 * when mounted inside the live application, preserving the pure header isolation in SSR tests.
 */
export function PersonaPortalControl({ personaSlot, meshSlot, tunnelSlot }) {
  if (typeof document === 'undefined') return null;
  return (
    <>
      {personaSlot ? createPortal(<PersonaToggle />, personaSlot) : null}
      {meshSlot ? createPortal(<FleetMeshIndicator />, meshSlot) : null}
      {tunnelSlot ? createPortal(<MultiInterfaceLauncher />, tunnelSlot) : null}
    </>
  );
}

/** Light / System / Dark segmented control (spec section 7). Selected state: raised surface, border and weight, plus aria-pressed (not colour alone). */
export function ThemeControl({ theme, onTheme, label = 'Colour theme' }) {
  return (
    <div className="seg theme-control" role="group" aria-label={label}>
      {THEME_OPTIONS.map((o) => (
        <button key={o.id} type="button" className="seg__btn" aria-pressed={theme === o.id} data-theme-option={o.id} onClick={() => onTheme(o.id)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

/**
 * Header (sticky, glass-1 chrome; materials and fallbacks are in app.css): ghost mark and product name, the connection status (glyph + words, fed by
 * `connKind` exactly as before), a compact theme menu (from 820 px) and the Menu button (below 820 px).
 * The theme menu is a native <details> whose panel is a SOLID popover: it opens from the glass header, and glass never sits on glass.
 * On phones the theme control and the gateway facts live inside the menu so the header stays one row (skill S16 Simplicity: common path first).
 * The header publishes its measured height as `--header-h`, so the sticky rail, the phone sheet and scroll-padding follow it when text is scaled (WCAG 1.4.4, 2.4.11).
 */
export function Header({ connKind, theme, onTheme, menuOpen = false, onMenu = () => {} }) {
  const [tone, icon] = KIND_PILL[connKind] || ['neutral', 'info'];
  const headerRef = useRef(null);
  const themeRef = useRef(null);
  useEffect(() => {
    const el = headerRef.current;
    if (!el || typeof ResizeObserver === 'undefined') return undefined;
    const root = document.documentElement;
    const publish = () => root.style.setProperty('--header-h', `${Math.ceil(el.getBoundingClientRect().height)}px`);
    publish();
    const ro = new ResizeObserver(publish);
    ro.observe(el);
    return () => {
      ro.disconnect();
      root.style.removeProperty('--header-h');
    };
  }, []);
  useEffect(() => {
    // Close the theme menu on an outside press or Escape; Escape returns focus to the button that opened it (skill S7).
    const onDown = (e) => {
      const d = themeRef.current;
      if (d && d.open && !d.contains(e.target)) d.open = false;
    };
    const onKey = (e) => {
      const d = themeRef.current;
      if (e.key === 'Escape' && d && d.open) {
        d.open = false;
        d.querySelector('summary')?.focus();
      }
    };
    document.addEventListener('pointerdown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('pointerdown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, []);
  const choose = (id) => {
    onTheme(id);
    const d = themeRef.current;
    if (d) {
      d.open = false;
      d.querySelector('summary')?.focus();
    }
  };
  return (
    <header className="app-header" ref={headerRef}>
      <div className="app-header__brand">
        <GhostMark />
        <span>PhantomCache<span className="app-header__brand-more"> Control Center</span></span>
      </div>
      <div className="app-header__persona-slot" />
      <div className="app-header__mesh-slot" />
      <div className="app-header__tunnel-slot" />
      <div className="app-header__status" data-conn-kind={connKind}>
        <StatusPill tone={tone} icon={icon}>{CONNECTION_TEXT[connKind]?.title || connKind}</StatusPill>
      </div>
      <div className="app-header__theme">
        <button
          type="button"
          className="app-header__theme-quick"
          onClick={() => choose(theme === 'dark' ? 'light' : 'dark')}
          title={`Switch to ${theme === 'dark' ? 'Light' : 'Dark'} mode`}
          aria-label={`Switch to ${theme === 'dark' ? 'Light' : 'Dark'} mode`}
        >
          <Icon name={theme === 'dark' ? 'sun' : 'moon'} />
          <span className="app-header__theme-quick-text">{theme === 'dark' ? 'Light' : 'Dark'}</span>
        </button>
        <details className="theme-menu" ref={themeRef}>
          <summary aria-label="Colour theme"><Icon name="theme" /><span className="visually-hidden">Colour theme</span></summary>
          <div className="popover theme-menu__panel">
            <ThemeControl theme={theme} onTheme={choose} />
          </div>
        </details>
      </div>
      {/* .btn is inline-flex, so the hide rule at 820 px must be more specific than .btn: `.app-header .app-nav__toggle`. */}
      <button type="button" className="btn app-nav__toggle" id={MENU_BUTTON_ID} aria-expanded={menuOpen} aria-controls={NAV_LIST_ID} onClick={onMenu}>
        Menu
      </button>
    </header>
  );
}

/**
 * The registry lists Intelligence under 'Not available yet'. Only when the gateway says the Phantom Intelligence API is installed does the
 * entry move up to 'Live gateway' (right after Cluster) and lose the 'Not available' tag. The count of screens does not change.
 */
function screensFor(intelligenceLive, demoMode = false) {
  if (demoMode) {
    return SCREENS.filter((s) => s.id === 'overview' || s.id === 'traffic');
  }
  if (!intelligenceLive) return SCREENS;
  const rest = SCREENS.filter((s) => s.id !== 'intelligence');
  const at = rest.findIndex((s) => s.id === 'cluster') + 1;
  const live = { ...SCREENS.find((s) => s.id === 'intelligence'), kind: 'data', group: 'Live gateway', tag: undefined };
  return [...rest.slice(0, at), live, ...rest.slice(at)];
}

function groupsOf(screens) {
  const groups = [];
  screens.forEach((s, index) => {
    let g = groups[groups.length - 1];
    if (!g || g.name !== s.group) {
      g = { name: s.group, items: [] };
      groups.push(g);
    }
    g.items.push({ ...s, index });
  });
  return groups;
}

const FOLD_KEY = 'phantom.ui.nav.unavailable-open';

function readFold() {
  try {
    const s = getStorage(typeof window !== 'undefined' ? window : null);
    return !(s && s.getItem(FOLD_KEY) === 'closed');
  } catch {
    return true;
  }
}
function writeFold(open) {
  try {
    const s = getStorage(typeof window !== 'undefined' ? window : null);
    if (s) s.setItem(FOLD_KEY, open ? 'open' : 'closed');
  } catch {
    /* the choice is simply not remembered */
  }
}

/**
 * Primary navigation. At 820 px and up it is the sticky glass rail; below 820 px it is a SOLID sheet opened by the header Menu button
 * (open state is held by the App so Escape can return focus to that button). Groups are real nested lists named for their contents.
 * The group whose entries are all "not available yet" folds into one disclosure that shows its count (open by default; the choice is remembered per
 * viewer in localStorage inside try/catch, and nothing else is persisted). Arrow keys still walk every link; a folded group opens when the walk enters it.
 */
let liveNavDemoMode = true;

export function Nav({ currentId, open = false, onNavigate = () => {}, theme = 'system', onTheme = () => {}, mode, gatewayUrl }) {
  // Whether the gateway serves the Phantom Intelligence API comes from the telemetry context (null outside the App: then the registry default applies).
  const tctx = useContext(TelemetryContext);
  const intelligenceLive = Boolean(tctx && tctx.intelLive);
  const refs = useRef([]);
  const [foldOpen, setFoldOpen] = useState(readFold);

  const [demoMode, setDemoMode] = useState(() => (typeof window !== 'undefined' ? liveNavDemoMode : false));

  const toggleDemoMode = () => {
    setDemoMode((prev) => {
      const next = !prev;
      liveNavDemoMode = next;
      return next;
    });
  };

  const currentScreens = screensFor(intelligenceLive, demoMode);
  const onKeyDown = (e, i) => {
    const next = nextNavIndex(e.key, i, currentScreens.length);
    if (next === null) return;
    e.preventDefault();
    const el = refs.current[next];
    const fold = el?.closest('details');
    if (fold && !fold.open) fold.open = true;
    el?.focus();
  };
  const groups = groupsOf(currentScreens);
  return (
    <nav className={`app-nav${open ? ' is-open' : ''}`} aria-label="Primary">
      <div className="app-nav__panel">
        <ul id={NAV_LIST_ID} className="app-nav__groups">
          {groups.map((g, gi) => {
            const collapsible = g.items.every((s) => s.kind === 'placeholder');
            const label = <span className="app-nav__group-label" id={`app-nav-group-${gi}`}>{g.name}</span>;
            const list = (
              <ul aria-labelledby={`app-nav-group-${gi}`}>
                {g.items.map((s) => (
                  <li key={s.id}>
                    <a
                      href={hrefFor(s.id)}
                      ref={(el) => { refs.current[s.index] = el; }}
                      aria-current={s.id === currentId ? 'page' : undefined}
                      onClick={() => onNavigate()}
                      onKeyDown={(e) => onKeyDown(e, s.index)}
                      data-screen={s.id}
                    >
                      <span>{s.label}</span>
                      {s.tag ? <span className="app-nav__tag" aria-hidden="true">{s.tag}</span> : null}
                      {s.kind === 'placeholder' ? <span className="visually-hidden"> (not yet available)</span> : null}
                      {s.kind === 'sim' ? <span className="visually-hidden"> (simulated)</span> : null}
                    </a>
                  </li>
                ))}
              </ul>
            );
            return (
              <li key={g.name} className="app-nav__group">
                {collapsible ? (
                  <details
                    className="app-nav__fold"
                    open={foldOpen}
                    onToggle={(e) => {
                      if (e.currentTarget.open !== foldOpen) {
                        setFoldOpen(e.currentTarget.open);
                        writeFold(e.currentTarget.open);
                      }
                    }}
                  >
                    <summary>
                      {label}
                      <span className="app-nav__group-count">{g.items.length}</span>
                    </summary>
                    {list}
                  </details>
                ) : (
                  <>
                    {label}
                    {list}
                  </>
                )}
              </li>
            );
          })}
        </ul>
        <div className="app-nav__foot">
          <div className="app-nav__foot-theme">
            <span className="app-nav__foot-label">Theme</span>
            <ThemeControl theme={theme} onTheme={onTheme} label="Colour theme (menu)" />
          </div>
          <div className="app-nav__foot-gateway">
            <span className="app-nav__foot-label">Gateway</span>
            <span className="mono" title="Gateway URL">{gatewayUrl || 'unknown'}</span>
            {mode ? <span>Reports mode: {mode}</span> : null}
          </div>
          <div className="app-nav__foot-gateway">
            <span className="app-nav__foot-label">Navigation</span>
            <button
              type="button"
              className="app-nav__mode-btn"
              onClick={toggleDemoMode}
              title="Toggle between focused demo showcase and all diagnostic screens"
            >
              {demoMode ? 'Switch to All Screens (10)' : 'Switch to Demo Showcase (2)'}
            </button>
          </div>
        </div>
      </div>
    </nav>
  );
}
