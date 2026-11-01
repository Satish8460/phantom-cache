import React, { useContext, useEffect, useRef, useState } from 'react';
import { TelemetryContext } from '../hooks/TelemetryContext.js';
import { StatusPill } from './States.jsx';
import Icon, { GhostMark } from './Icon.jsx';
import { SCREENS, hrefFor, nextNavIndex } from '../lib/routes.js';
import { CONNECTION_TEXT } from '../lib/connection.js';
import { THEME_OPTIONS, getStorage } from '../lib/theme.js';

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
      <div className="app-header__status" data-conn-kind={connKind}>
        <StatusPill tone={tone} icon={icon}>{CONNECTION_TEXT[connKind]?.title || connKind}</StatusPill>
      </div>
      <div className="app-header__theme">
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
function screensFor(intelligenceLive) {
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
export function Nav({ currentId, open = false, onNavigate = () => {}, theme = 'system', onTheme = () => {}, mode, gatewayUrl }) {
  // Whether the gateway serves the Phantom Intelligence API comes from the telemetry context (null outside the App: then the registry default applies).
  const tctx = useContext(TelemetryContext);
  const intelligenceLive = Boolean(tctx && tctx.intelLive);
  const refs = useRef([]);
  const [foldOpen, setFoldOpen] = useState(readFold);
  const onKeyDown = (e, i) => {
    const next = nextNavIndex(e.key, i, SCREENS.length);
    if (next === null) return;
    e.preventDefault();
    const el = refs.current[next];
    const fold = el?.closest('details');
    if (fold && !fold.open) fold.open = true;
    el?.focus();
  };
  const groups = groupsOf(screensFor(intelligenceLive));
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
        </div>
      </div>
    </nav>
  );
}
