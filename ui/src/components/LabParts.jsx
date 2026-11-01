import React, { useEffect, useId, useMemo, useState } from 'react';
import Icon from './Icon.jsx';
import ProvenanceBadge from './ProvenanceBadge.jsx';
import { StatusPill } from './States.jsx';
import { runStateInfo, tierBadgeQualifier, tierInfo, verdictInfo, labelInfo } from '../lib/labProvenance.js';
import { parseMarkdown, renderMarkdown } from '../lib/miniMarkdown.js';

/**
 * Provenance badge for an API tier string (MEASURED_LOOPBACK, SIMULATED, RETAINED_*, DERIVED, UNAVAILABLE).
 * Display words follow the owner vocabulary: measured tiers show REAL with the basis qualifier ("loopback, one host", plus
 * "debug build" when `build` says so); RETAINED_* adds a separate neutral "Retained" tag instead of changing the word.
 */
export function TierBadge({ tier, build }) {
  const t = typeof tier === 'object' && tier !== null ? tier : tierInfo(tier);
  return (
    <>
      <ProvenanceBadge src={t.src} label={t.word || t.label} qualifier={tierBadgeQualifier(t, build) || undefined} title={t.meaning} tier={t.key} />
      {t.retained ? <Stamp /> : null}
    </>
  );
}

/**
 * RETAINED indicator (DEC-007, DD v2 8.2): a separate axis from the provenance word, never animated, never "live".
 * It says the experiment was read from an earlier folder and this page only displays it.
 */
export function Stamp() {
  return (
    <span className="stamp" data-retained title="Read from a retained experiment folder; this page only displays it.">
      <Icon name="retained" />
      Retained
    </span>
  );
}

/** Run state chip: glyph + word, never colour alone. */
export function StateChip({ state, suffix }) {
  const s = runStateInfo(state);
  return (
    <span data-run-state={s.key} data-state-qualified={suffix ? 'true' : undefined} title={s.meaning}>
      <StatusPill tone={suffix ? 'warn' : s.tone} icon={suffix ? 'warn' : s.icon}>{suffix ? `${s.label} ${suffix}` : s.label}</StatusPill>
    </span>
  );
}

/** One component for every verdict, so negative and inconclusive outcomes have the same size and weight as positive ones. */
export function VerdictChip({ outcome, name }) {
  const v = typeof outcome === 'object' && outcome !== null && outcome.label ? outcome : verdictInfo(outcome);
  return (
    <span className="verdict" data-verdict={v.key} data-verdict-kind={v.kind}>
      {name ? <span className="verdict__name">{name}</span> : null}
      <StatusPill tone={v.tone} icon={v.icon}>{v.label}</StatusPill>
    </span>
  );
}

export function LabelChip({ label }) {
  const l = labelInfo(label);
  return (
    <span data-exp-label={l.key} className="verdict">
      <StatusPill tone={l.tone} icon={l.icon}>{l.label}</StatusPill>
      {l.hint ? <span className="lab-hint" data-run-class-hint>{l.hint}</span> : null}
    </span>
  );
}

/** Banner for notes that must be read: tone is a glyph + word + border, not only colour. Use sparingly (two rows at most above a task). */
export function Notice({ tone = 'info', title, children, role = 'note', id }) {
  const icon = { info: 'info', warn: 'warn', critical: 'error', ok: 'ok' }[tone] || 'info';
  return (
    <div className={`banner tone-${tone}`} role={role} id={id} data-notice={tone}>
      <Icon name={icon} />
      <div className="banner__body">
        {title ? <span className="banner__title">{title}</span> : null}
        <div>{children}</div>
      </div>
    </div>
  );
}

/** Table-cell content for a view-model cell. Unavailable cells say so in words and give the reason. */
export function CellText({ c }) {
  if (!c) return <span>Not reported</span>;
  if (c.state === 'value') return <span className="num">{c.text}</span>;
  return (
    <span className="lab-unavailable">
      Unavailable<span className="visually-hidden">: {c.reason}</span>
      <span className="lab-unavailable__reason" aria-hidden="true"> ({c.reason})</span>
    </span>
  );
}

/** A value plus its tier badge, for stand-alone figures. */
export function Figure({ label, cellValue, tier, hint, build }) {
  const t = tier || 'UNAVAILABLE';
  return (
    <div className="lab-figure">
      <dt>{label}</dt>
      <dd className="lab-figure__value"><CellText c={cellValue} /></dd>
      <dd className="lab-figure__meta"><TierBadge tier={cellValue && cellValue.state === 'unavailable' ? 'UNAVAILABLE' : t} build={build} />{hint ? <span>{hint}</span> : null}</dd>
    </div>
  );
}

export function KeyValues({ rows, label }) {
  return (
    <dl className="kv" aria-label={label}>
      {rows.map((r) => (
        <React.Fragment key={r.key || r.label}>
          <dt>{r.label}</dt>
          <dd>{r.node ?? r.text}</dd>
        </React.Fragment>
      ))}
    </dl>
  );
}

/** Preformatted wrapped text for verbatim payload text (the Raw notice view, the exact request, the terminal command). */
export function VerbatimText({ text, label }) {
  return <pre className="lab-verbatim" aria-label={label} tabIndex={0}>{text}</pre>;
}

/** Copy to clipboard with an inline status (role=status). Nothing is copied unless the person presses the button. */
export function CopyButton({ text, label }) {
  const [state, setState] = useState('idle');
  return (
    <>
      <button
        type="button" className="btn"
        onClick={async () => {
          try {
            await navigator.clipboard.writeText(text);
            setState('copied');
          } catch {
            setState('failed');
          }
        }}
      >
        {label}
      </button>
      <span role="status" className="lab-hint">{state === 'copied' ? 'Copied.' : state === 'failed' ? 'Copy is not available here; select the text instead.' : ''}</span>
    </>
  );
}

// ---------------------------------------------------------------------------------------------------------------
// Sections (spec section 4: headings and spacing instead of boxes inside boxes)

/** A page section: rule + heading + optional note. Not a card unless `card` (one L1 surface for the result of a run or experiment). `level` is the heading element (default h2). */
export function LabSection({ id, title, note, aside, level = 2, card = false, children }) {
  const hid = useId();
  const H = `h${level}`;
  return (
    <section className={card ? 'lab-section lab-section--card' : 'lab-section'} id={id} aria-labelledby={hid} data-lab-section={id || undefined}>
      <div className="lab-section__head">
        <H id={hid} tabIndex={-1} className="lab-section__title">{title}</H>
        {aside ? <div className="lab-section__aside">{aside}</div> : null}
      </div>
      {note ? <p className="lab-section__note">{note}</p> : null}
      {children}
    </section>
  );
}

/**
 * A collapsible section for operational detail (workload settings, files, manifest, event log). The summary holds the h2
 * and a count, so a closed fold still says what it contains. Binding wording is never put inside a closed fold.
 */
export function FoldSection({ id, title, count, note, defaultOpen = false, children }) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <details className="disclosure lab-fold" id={id} open={open} onToggle={(e) => setOpen(e.currentTarget.open)} data-lab-section={id || undefined} data-fold>
      <summary className="lab-fold__summary">
        <h2 className="lab-fold__title">{title}</h2>
        {count ? <span className="lab-fold__count">{count}</span> : null}
      </summary>
      <div className="lab-fold__body">
        {note ? <p className="lab-section__note">{note}</p> : null}
        {children}
      </div>
    </details>
  );
}

/**
 * "On this page" list. Hash routing owns location.hash, so these anchors scroll and move focus WITHOUT changing the hash
 * (preventDefault); a closed fold is opened first. Styled as a sticky rail on wide screens and a sticky bar under the header
 * below that (lab.css). The section in view is marked aria-current="location" (IntersectionObserver, no scroll listener; where
 * the API is missing nothing is marked).
 */
export function ContentsNav({ items, sticky = true }) {
  const list = items.filter(Boolean);
  const [active, setActive] = useState(null);
  const key = list.map((i) => i.id).join('|');
  useEffect(() => {
    if (typeof IntersectionObserver !== 'function' || list.length < 3) return undefined;
    const els = list.map((i) => document.getElementById(i.id)).filter(Boolean);
    if (!els.length) return undefined;
    const seen = new Map();
    const io = new IntersectionObserver((entries) => {
      for (const e of entries) seen.set(e.target.id, e.isIntersecting ? e.boundingClientRect.top : null);
      const visible = [...seen.entries()].filter(([, top]) => top !== null).sort((a, b) => a[1] - b[1]);
      if (visible.length) setActive(visible[0][0]);
    }, { rootMargin: '-15% 0px -70% 0px' });
    els.forEach((el) => io.observe(el));
    return () => io.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  if (list.length < 3) return null;
  const go = (e, id) => {
    e.preventDefault();
    const el = document.getElementById(id);
    if (!el) return;
    if (el.tagName === 'DETAILS') el.open = true;
    el.scrollIntoView({ block: 'start' });
    const target = el.tagName === 'DETAILS' ? el.querySelector('summary') : el.querySelector('[tabindex="-1"]');
    if (target) target.focus({ preventScroll: true });
    setActive(id);
  };
  return (
    <nav className="lab-toc" aria-label="On this page" data-contents data-sticky={sticky ? 'bar' : 'rail-only'}>
      <span className="lab-toc__label">On this page</span>
      <ul>
        {list.map((it) => <li key={it.id}><a href={`#${it.id}`} onClick={(e) => go(e, it.id)} aria-current={active === it.id ? 'location' : undefined}>{it.label}</a></li>)}
      </ul>
    </nav>
  );
}

/**
 * Page body with a contents list. DOM order: contents first, then the content. On wide screens CSS places the list in a sticky
 * right-hand column (12 rem); below that it is a sticky bar under the header.
 */
export function RailLayout({ toc, children, stickyBar = true }) {
  return (
    <div className="lab-rail-layout">
      <ContentsNav items={toc} sticky={stickyBar} />
      <div className="lab-rail-layout__main">{children}</div>
    </div>
  );
}

/**
 * Binding text from the payload (the interpretation notice). Rendered view by default (miniMarkdown: headings, bold, lists,
 * code; links disabled), with a Raw view that is the payload string unchanged and a Copy button. Only the active view is in
 * the DOM, so the rendered view never contains raw markers and find-in-page does not see the text twice.
 */
export function RichNotice({ text, label, defaultView = 'rendered' }) {
  const parsed = useMemo(() => parseMarkdown(text), [text]);
  const [view, setView] = useState(defaultView);
  const canRender = parsed.ok;
  const active = canRender ? view : 'raw';
  return (
    <div className="lab-notice" data-rich-notice data-notice-view={active}>
      <div className="lab-notice__bar">
        <div className="seg" role="group" aria-label="Notice view">
          <button type="button" className="seg__btn" aria-pressed={active === 'rendered'} disabled={!canRender} onClick={() => setView('rendered')}>Rendered</button>
          <button type="button" className="seg__btn" aria-pressed={active === 'raw'} onClick={() => setView('raw')}>Raw</button>
        </div>
        <CopyButton text={text} label="Copy text" />
        {!canRender ? <span className="lab-hint">Shown raw: the text is longer than the renderer accepts. Nothing is cut.</span> : null}
      </div>
      {active === 'rendered' ? (
        <div className="prose lab-notice__body" role="region" aria-label={label} data-notice-pane="rendered">{renderMarkdown(parsed.blocks, React.createElement, { headingBase: 3 })}</div>
      ) : (
        <pre className="lab-verbatim" aria-label={label} tabIndex={0} data-notice-pane="raw">{text}</pre>
      )}
    </div>
  );
}
