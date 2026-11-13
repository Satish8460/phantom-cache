import React, { useEffect, useId, useRef, useState } from 'react';
import { StateBlock, Spinner } from '../../components/States.jsx';
import Icon from '../../components/Icon.jsx';
import ProvenanceBadge from '../../components/ProvenanceBadge.jsx';
import { CellText, FoldSection, KeyValues, LabSection, Notice, RailLayout, StateChip, TierBadge } from '../../components/LabParts.jsx';
import ScenarioBResult, { sbContents } from './ScenarioBResult.jsx';
import { ErrorBlock } from './Gate.jsx';
import { viewAsSummary } from './labCopy.js';
import { labHref } from '../../lib/routes.js';
import { describeRunStreamLink } from '../../lib/linkState.js';
import { elapsedView, t1ResultView, resultEventLabel } from '../../lib/labView.js';
import { progressView } from '../../lib/progress.js';
import { applyPresentation, resultPresentation } from '../../lib/resultPresentation.js';
import { formatCount } from '../../lib/format.js';
import { runStateInfo } from '../../lib/labProvenance.js';

// Runs list and run detail (increment I2). Run detail order: sticky run header (name, id, state, REAL/SIMULATED tier with its
// qualifier, elapsed, progress, live-update status, Cancel), notices, then Progress while active or "Result at a glance"
// when terminal, artifacts, settings, event log. Everything shown comes from the run record, its events or its timestamps:
// elapsed time is labelled DERIVED (from timestamps) or UNAVAILABLE with the reason; phase and arm progress is shown only as
// far as the progress events say it (no invented steps).
// Skill: S16 (status, completion, warning, error feedback; Agency: confirm only the destructive action), S1 (instant feedback),
// S14 (reduced motion handled by the shared CSS), spec section 4 (L2 sticky header, L3 dialog).

// ---------------------------------------------------------------------------------------------------------------
// Runs list

export function RunsListBody({ view, onRefresh }) {
  if (!view.items.length) {
    return (
      <StateBlock kind="empty" title="No runs yet" action={<a className="btn" href={labHref('run')}>Start a run</a>}>
        The gateway has no run history. This is not a failure.
      </StateBlock>
    );
  }
  return (
    <LabSection
      id="runs" title="Run history"
      note={`Newest first. ${view.returned} shown of ${view.total} indexed${view.boundedTo === null ? '' : ` (the index covers the newest ${view.boundedTo})`}.`}
      aside={onRefresh ? <button type="button" className="btn" onClick={onRefresh}>Refresh</button> : null}
    >
      <div className="table-wrap">
        <table data-runs-table>
          <caption>Runs started through the Lab API</caption>
          <thead>
            <tr><th scope="col">Run</th><th scope="col">Kind</th><th scope="col">State</th><th scope="col">Created</th><th scope="col">Progress</th><th scope="col">Provenance</th></tr>
          </thead>
          <tbody>
            {view.items.map((r) => (
              <tr key={r.id} data-run-id={r.id}>
                <th scope="row"><a href={r.href} className="mono">{r.id}</a></th>
                <td>{r.kind.short}</td>
                <td><StateChip state={r.state} /></td>
                <td>{r.created}</td>
                <td className="num">{r.progress.done === null || r.progress.total === null ? 'not reported' : `${formatCount(r.progress.done)} of ${formatCount(r.progress.total)}`}</td>
                <td><TierBadge tier={r.tier} build={r.buildProfile} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </LabSection>
  );
}

// ---------------------------------------------------------------------------------------------------------------
// Run detail pieces

/** Browser clock tick for the elapsed time of an active run. One update per second; nothing ticks once the run is terminal. */
function useNow(active) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!active) return undefined;
    setNow(Date.now());
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, [active]);
  return now;
}

const PROBLEM_STREAM = ['stale', 'reconnecting', 'disconnected', 'failed'];

/**
 * Full stream banner, only for the four problem states (stale, reconnecting, disconnected, failed) or when event numbers had a gap
 * (the list may be incomplete, which is worth saying even while the stream is live). Normal states are a small chip.
 * Words, cause and the retry verb come from linkState (one vocabulary on every surface); the banner adds only the stream facts.
 */
function StreamBanner({ live, link, onReconnect }) {
  const problem = PROBLEM_STREAM.includes(live.status);
  if (!problem && !(live.stream.gaps > 0)) return null;
  const s = live.stream;
  const bits = [];
  if (s.lastEventId !== null) bits.push(`Last event received: number ${s.lastEventId}.`);
  if (s.gaps > 0) bits.push(`${s.gaps} gap(s) in event numbers were seen; the event list may be incomplete.`);
  return (
    <div className="banner tone-warn" role="status" data-stream-status={live.status}>
      <Icon name="warn" />
      <div className="banner__body">
        <span className="banner__title">Live updates: {link.word}</span>
        {problem ? <span>{link.cause}</span> : null}
        {problem && link.detail ? <span>{link.detail}</span> : null}
        {problem && link.fix ? <span>{link.fix}</span> : null}
        {bits.length ? <span>{bits.join(' ')}</span> : null}
        {problem && link.retry ? <span><button type="button" className="btn" onClick={onReconnect}>{link.retryLabel}</button></span> : null}
      </div>
    </div>
  );
}

/** Small status, not a banner: the live-update state as glyph + words. */
function LiveChip({ status, link }) {
  if (!link || !link.word) return null;
  const icon = status === 'live' ? 'ok' : status === 'ended' ? 'ok' : PROBLEM_STREAM.includes(status) ? 'warn' : 'clock';
  return (
    <span className="lab-live" data-live-chip={status} title={link.cause}>
      <Icon name={icon} />
      <span>Live updates: {link.word.toLowerCase()}</span>
    </span>
  );
}

function ElapsedChip({ elapsed }) {
  if (elapsed.state === 'value') {
    return (
      <span className="lab-elapsed" data-elapsed title={`Computed in this page from the ${elapsed.basis}.`}>
        <span className="lab-elapsed__label">Elapsed</span> <span className="num">{elapsed.text}</span>
        <ProvenanceBadge src="derived" qualifier="from timestamps" />
      </span>
    );
  }
  return (
    <span className="lab-elapsed" data-elapsed="unavailable">
      <span className="lab-elapsed__label">Elapsed</span> <span className="lab-unavailable">Unavailable</span>
      <ProvenanceBadge src="unavailable" />
      <span className="visually-hidden">: {elapsed.reason}</span>
      <span className="lab-hint" aria-hidden="true">({elapsed.reason})</span>
    </span>
  );
}

/** Header progress: a bar only when it carries information (never 0 of N); otherwise the plain count, or nothing. */
function MiniProgress({ pv }) {
  if (!pv.counts) return null;
  const label = `${formatCount(pv.counts.done)} of ${formatCount(pv.counts.total)} ${pv.unitPlural}`;
  return (
    <span className="lab-miniprogress" data-mini-progress>
      {pv.bar.render ? <progress value={pv.bar.value} max={pv.bar.max} aria-label="Run progress">{label}</progress> : null}
      <span className="num">{label}</span>
    </span>
  );
}

/**
 * Cancel confirmation as a NATIVE modal dialog (L3): <dialog> opened with showModal(), so the browser traps focus, makes the
 * rest of the page inert and closes it on Escape (the `cancel` event). Focus starts on the safe choice ("Keep running"). Escape does
 * nothing while the request is being sent. The caller returns focus to the Cancel button (or the run heading when it is gone).
 */
function CancelDialog({ phase, onConfirm, onAbort }) {
  const tid = useId();
  const did = useId();
  const keepRef = useRef(null);
  const boxRef = useRef(null);
  const sending = phase === 'sending';
  useEffect(() => {
    const el = boxRef.current;
    if (el && typeof el.showModal === 'function' && !el.open) el.showModal();
    keepRef.current?.focus();
  }, []);
  return (
    <dialog
      className="dialog lab-dialog" aria-labelledby={tid} aria-describedby={did} ref={boxRef} data-cancel-dialog
      onCancel={(e) => { e.preventDefault(); if (!sending) onAbort(); }}
    >
      <h2 id={tid} className="dialog__title">Cancel this run?</h2>
      <p id={did}>It stops, and any result so far is partial. This cannot be undone; start a new run to try again.</p>
      <div className="dialog__actions">
        <button type="button" className="btn" ref={keepRef} onClick={onAbort} disabled={sending}>Keep running</button>
        <button type="button" className="btn btn--danger" onClick={onConfirm} disabled={sending} aria-busy={sending ? 'true' : undefined}>{sending ? 'Cancelling' : 'Yes, cancel the run'}</button>
      </div>
    </dialog>
  );
}

/**
 * Honest progress (lib/progress.js): counts are the gateway's; the bar is drawn only once at least one unit is done (never an empty
 * bar for 0 of N); "at least X left" is a lower bound from the planned phase lengths and is marked DERIVED; there is no ETA.
 * The indeterminate bar is looping motion and is shown only while the live stream is healthy.
 */
function ProgressBlock({ pv, frozen }) {
  const ph = pv.phase;
  return (
    <div className="lab-progress" data-progress>
      {pv.bar.render ? (
        <progress value={pv.bar.value} max={pv.bar.max} aria-label="Run progress">{formatCount(pv.bar.value)} of {formatCount(pv.bar.max)} {pv.unitPlural}</progress>
      ) : pv.terminal || frozen || pv.counts ? null : (
        <div className="progress-indeterminate" aria-hidden="true" />
      )}
      <p className="lab-progress__text" data-progress-text>{pv.text}</p>
      {ph ? <p data-progress-phase>{ph.step ? `Phase ${ph.step} of ${ph.steps}: ${ph.label}` : `Phase: ${ph.label}`}</p> : null}
      {pv.atLeastLeft.state === 'value' ? (
        <p data-at-least-left>{pv.atLeastLeft.text} <ProvenanceBadge src="derived" qualifier="from the planned phase lengths" /> <span className="lab-hint">{pv.atLeastLeft.basis}.</span></p>
      ) : pv.atLeastLeft.state === 'overdue' ? (
        <p data-at-least-left="overdue">{pv.atLeastLeft.text} <span className="lab-hint">({pv.atLeastLeft.basis})</span></p>
      ) : pv.terminal ? null : (
        <p className="lab-hint" data-at-least-left="unavailable">Time left: unavailable, because {pv.atLeastLeft.reason}. This page shows no estimated finish time.</p>
      )}
      <p className="lab-hint" data-unit-definition>{pv.unitDefinition}</p>
    </div>
  );
}

/** The last few events, always visible; the full list (up to the page's cap) is one disclosure away. */
function EventTail({ log, malformed, tail = 5 }) {
  if (!log.length) return <p className="panel__note" data-no-events>No events received by this page yet.</p>;
  const last = log.slice(-tail);
  return (
    <div className="lab-tail" data-event-tail>
      <h3 className="lab-subhead">Latest events</h3>
      <ol className="lab-log" aria-label="Latest events, oldest first" aria-live="off">
        {last.map((e) => <li key={`${e.id}-${e.event}`}><span className="mono">#{e.id === null ? '?' : e.id}</span> {e.summary}</li>)}
      </ol>
      {log.length > last.length ? (
        <details className="disclosure">
          <summary>Show all {log.length} events received</summary>
          <ol className="lab-log lab-log--all" aria-label="All events received, oldest first">
            {log.map((e) => <li key={`all-${e.id}-${e.event}`}><span className="mono">#{e.id === null ? '?' : e.id}</span> {e.summary}</li>)}
          </ol>
        </details>
      ) : null}
      {malformed ? <p className="panel__note">{malformed} event(s) could not be read and were skipped.</p> : null}
    </div>
  );
}

function ArtifactsTable({ art, onDownload, downloading, downloadError }) {
  if (art.state === 'loading') return <Spinner>Loading the artifact list</Spinner>;
  if (art.state === 'error') return <ErrorBlock error={art.error} what="artifacts" />;
  if (!art.rows.length) return <p className="panel__note" data-no-artifacts>No artifacts yet. They are listed once the runner has written files.</p>;
  return (
    <>
      <div className="table-wrap">
        <table data-artifacts-table>
          <caption>Artifacts of this run (allowlisted names; SHA-256 as recorded by the gateway)</caption>
          <thead><tr><th scope="col">Name</th><th scope="col">Size</th><th scope="col">SHA-256</th><th scope="col">Hash covers</th><th scope="col">Provenance</th><th scope="col">Download</th></tr></thead>
          <tbody>
            {art.rows.map((a) => (
              <tr key={a.name} data-artifact={a.name}>
                <th scope="row" className="mono">{a.name}</th>
                <td className="num">{a.bytes}</td>
                <td className="mono lab-hash">{a.sha256.state === 'value' ? a.sha256.text : <CellText c={{ state: 'unavailable', reason: a.sha256.reason }} />}</td>
                <td>{a.sha256Of || 'not reported'}</td>
                <td>{a.tier ? <TierBadge tier={a.tier} /> : <TierBadge tier="UNAVAILABLE" />}</td>
                <td>
                  {a.downloadable ? (
                    <button type="button" className="btn" onClick={() => onDownload(a.name)} disabled={downloading === a.name} aria-label={`Download ${a.name}`}>{downloading === a.name ? 'Downloading' : 'Download'}</button>
                  ) : 'Not downloadable'}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {downloadError ? <ErrorBlock error={downloadError} what="the download" /> : null}
    </>
  );
}

function t1Glance(resultEvent, build) {
  const r = t1ResultView(resultEvent);
  if (!r) return null;
  return (
    <LabSection id="run-result" title="Result at a glance" card aside={<TierBadge tier={r.tier} build={build} />} note={r.note || undefined}>
      <dl className="kv" aria-label="Simulation result">
        <dt>Cells run</dt><dd><CellText c={r.cells} /></dd>
        <dt>cells.csv SHA-256</dt><dd className="mono lab-hash">{r.cellsSha256 || 'not reported'}</dd>
      </dl>
      <Notice tone="info" title="No summary statistics for matrix runs" role="note">
        The Lab API reports a simulation matrix as a cell count and the hash of cells.csv. It computes no paired differences or verdict for it. Download cells.csv from the artifacts below to analyse the cells yourself.
      </Notice>
    </LabSection>
  );
}

/** Contents entries for the parts of a run page that come after the result. */
const RUN_TAIL_CONTENTS = [{ id: 'run-artifacts', label: 'Artifacts' }, { id: 'run-settings', label: 'Settings used' }, { id: 'run-events', label: 'Event log' }];

function ResultSection({ run, live, summary, p }) {
  const state = live.model.state || run.state;
  const info = runStateInfo(state);
  const kind = run.kind.key;
  const resultEvent = live.model.result;
  if (kind === 'scenario_b') {
    if (summary.state === 'ready') {
      // U-01: the rule decides; the view is stripped of verdicts and mechanism pass/fail when it forbids them (defence in depth).
      const view = applyPresentation(summary.view, p);
      return <ScenarioBResult view={view} presentation={p} valueTier="MEASURED_LOOPBACK" build={run.buildProfile} />;
    }
    if (summary.state === 'loading') return <StateBlock kind="loading" title="Loading the result summary">Fetching summary.json with your credentials.</StateBlock>;
    if (summary.state === 'error') return <ErrorBlock error={summary.error} what="the result summary" />;
    const lbl = resultEventLabel(resultEvent);
    if (info.terminal) {
      return p.mode === 'no_verdict' ? (
        <StateBlock kind="unavailable" title={p.headline}>
          {p.detail} No summary was read for this run, and nothing is shown in its place.
        </StateBlock>
      ) : (
        <StateBlock kind="unavailable" title="No result summary for this run">
          {state === 'completed' ? 'The run completed but the summary is not available.' : `The run ended as ${info.label.toLowerCase()} before a summary was written.`} Nothing is shown in its place.
        </StateBlock>
      );
    }
    return (
      <p className="panel__note" role="status" data-result-waiting>
        The result appears here when the run completes. While it runs, the interpretation text below already applies.
        {lbl && lbl.labelMeaning ? <> {lbl.labelMeaning}</> : null}
      </p>
    );
  }
  // T1 matrix
  const glance = p.mode === 'matrix' ? t1Glance(resultEvent, run.buildProfile) : null;
  if (glance) return glance;
  return info.terminal
    ? <StateBlock kind="unavailable" title={p.mode === 'no_verdict' && p.headline ? p.headline : 'No result recorded'}>The run ended as {info.label.toLowerCase()}{p.mode === 'no_verdict' ? '' : ' without a result event'}. Nothing is shown in its place.</StateBlock>
    : <p className="panel__note" role="status" data-result-waiting>The result appears here when the run completes.</p>;
}

/**
 * Run detail. All data and handlers arrive as props so the whole page renders on the server in tests.
 * `nowMs` (optional) fixes the clock for tests; otherwise a one second tick runs while the run is active.
 */
export function RunDetailBody({ run, live, artifacts, summary, cancel, onCancelStart, onCancelConfirm, onCancelAbort, onReconnect, onRefresh, onDownload, downloading, downloadError, nowMs }) {
  const state = live.model.state || run.state;
  const info = runStateInfo(state);
  const err = live.model.error || run.error;
  const tick = useNow(!info.terminal && nowMs === undefined);
  const now = nowMs !== undefined ? nowMs : tick;
  const elapsed = elapsedView({ startedMs: run.startedMs, endedMs: run.endedMs, nowMs: now, terminal: info.terminal });
  const link = describeRunStreamLink(live.stream, live.status, now);
  // A non-final state may be out of date while the stream is down: say so in the chip itself.
  const streamDown = PROBLEM_STREAM.includes(live.status);
  const staleSuffix = streamDown && !info.terminal
    ? `(last known; stream ${live.status}${live.stream.lastEventId === null ? ', no event received' : `, last event ${live.stream.lastEventId}`})`
    : null;
  const cancelRef = useRef(null);
  const headingRef = useRef(null);
  const confirming = cancel.phase === 'confirm' || cancel.phase === 'sending';
  // Focus returns to the Cancel button when the dialog closes, or to the run heading when that button is gone (cancel requested).
  const wasConfirming = useRef(false);
  useEffect(() => {
    if (wasConfirming.current && !confirming) {
      const el = cancelRef.current && cancelRef.current.isConnected ? cancelRef.current : headingRef.current;
      if (el) el.focus();
    }
    wasConfirming.current = confirming;
  }, [confirming]);
  const cancelRequested = state === 'cancelling' || cancel.phase === 'sent';
  const progress = live.model.progress || (run.progress.total !== null ? { done: run.progress.done, total: run.progress.total } : null);
  const pv = progressView({
    kind: run.kind.key, state, progress, spec: run.specRaw, startedMs: run.startedMs, endedMs: run.endedMs, nowMs: now, lastEventAtMs: live.stream.lastActivityAt,
  });
  // U-01: one rule decides whether a verdict, a mechanism verdict or a run-class label may be shown (lib/resultPresentation.js).
  const rawSummary = summary.state === 'ready' ? (summary.raw || viewAsSummary(summary.view)) : null;
  const p = resultPresentation(state, rawSummary, { runKind: run.kind.key, progress: progress ? { done: progress.done, total: progress.total } : run.progress });
  const toc = run.kind.key === 'scenario_b' && summary.state === 'ready' ? [...sbContents(summary.view, p), ...RUN_TAIL_CONTENTS] : [];
  const showWaitingPolicy = run.interpretation && run.kind.key === 'scenario_b' && !live.model.result;
  return (
    <RailLayout toc={toc} stickyBar={false}>
      <div className="lab-detail-head lab-run-head" data-run-detail={run.id}>
        <div className="lab-run-head__top">
          <div className="lab-run-head__title">
            <h2 className="lab-run-head__kind" tabIndex={-1} ref={headingRef}>{run.kind.label}</h2>
            <span className="mono lab-run-head__id">{run.id}</span>
          </div>
          <div className="lab-run-head__actions">
            <button type="button" className="btn btn--ghost" onClick={onRefresh}>Refresh</button>
            {!info.terminal && !cancelRequested ? (
              <button type="button" className="btn btn--danger" ref={cancelRef} onClick={onCancelStart} aria-haspopup="dialog" data-cancel-open>Cancel run</button>
            ) : null}
          </div>
        </div>
        <div className="lab-run-head__meta">
          <StateChip state={state} suffix={staleSuffix} />
          <TierBadge tier={run.tier} build={run.buildProfile} />
          <ElapsedChip elapsed={elapsed} />
          <MiniProgress pv={pv} />
          <LiveChip status={live.status} link={link} />
        </div>
        <p className="visually-hidden" role="status" data-run-status-live>{`Run ${info.label}. Live updates: ${link.word.toLowerCase()}.`}</p>
      </div>
      {live.model.recovery || state === 'interrupted' ? (
        <Notice tone="warn" title="Interrupted by a gateway restart" role="status">
          The gateway stopped or restarted while this run was active. The run was not resumed, has no result, and the server wrote the interrupted marks itself after the restart.
        </Notice>
      ) : null}
      {err && state === 'failed' ? (
        <Notice tone="critical" title={`The run failed (${err.code})`} role="alert">{err.message || 'The gateway gave no message.'}</Notice>
      ) : null}
      {state === 'cancelled' ? (
        <Notice tone="warn" title="This run was cancelled" role="status">Stopped on request. Any artifact is partial and any summary has unavailable fields.</Notice>
      ) : null}
      {cancelRequested && !info.terminal ? (
        <Notice tone="warn" title="Cancel requested" role="status">The gateway accepted the request. The run stops cooperatively; a simulation stops at the next cell boundary. Artifacts that exist afterwards are partial.</Notice>
      ) : null}
      {cancel.phase === 'error' && cancel.error ? <ErrorBlock error={cancel.error} what="cancelling" /> : null}
      <StreamBanner live={live} link={link} onReconnect={onReconnect} />
      {run.persistenceErrors ? <Notice tone="warn" title="Persistence errors" role="status">The gateway counted {run.persistenceErrors} failed write(s) of this run record. The history may be incomplete.</Notice> : null}

      {!info.terminal ? (
        <LabSection id="run-progress" title="Progress">
          <ProgressBlock pv={pv} frozen={streamDown} />
          <EventTail log={live.model.log} malformed={live.model.malformed} />
          <ResultSection run={run} live={live} summary={summary} p={p} />
        </LabSection>
      ) : (
        <ResultSection run={run} live={live} summary={summary} p={p} />
      )}

      {showWaitingPolicy ? (
        <LabSection id="run-howto" title="How to read the result when it arrives">
          {run.interpretation.labelMeaning ? <p>{run.interpretation.labelMeaning}</p> : null}
          <ul className="lab-reasons">{run.interpretation.limitations.map((l) => <li key={l}>{l}</li>)}</ul>
        </LabSection>
      ) : null}

      <LabSection id="run-artifacts" title="Artifacts" note="Sizes and SHA-256 as recorded by the gateway. Downloads use your credentials in a request header, never in the address.">
        <ArtifactsTable art={artifacts} onDownload={onDownload} downloading={downloading} downloadError={downloadError} />
      </LabSection>
      <FoldSection id="run-settings" title="Settings used" count={`${run.spec.length} value(s)`} note="These are the values the gateway echoed, including defaults it filled in.">
        <KeyValues rows={run.spec.map((r) => ({ key: r.key, label: r.label, text: r.text }))} label="Run settings" />
        <dl className="kv" aria-label="Run times">
          <dt>Created</dt><dd>{run.created}</dd>
          <dt>Started</dt><dd>{run.started}</dd>
          <dt>Ended</dt><dd>{run.ended}</dd>
          <dt>Exit code</dt><dd>{run.exitCode === null ? 'none yet' : run.exitCode}</dd>
          <dt>Gateway build</dt><dd>{run.buildProfile || 'not reported'}</dd>
          <dt>Events received</dt><dd>{live.stream.eventCount}</dd>
        </dl>
      </FoldSection>
      {info.terminal ? (
        <FoldSection id="run-events" title="Event log" count={`${live.model.log.length} event(s)`}>
          {live.model.log.length ? (
            <ol className="lab-log" aria-label="Events received, oldest first">
              {live.model.log.map((e) => <li key={`${e.id}-${e.event}`}><span className="mono">#{e.id === null ? '?' : e.id}</span> {e.summary}</li>)}
            </ol>
          ) : <p className="panel__note">No events received by this page yet.</p>}
          {live.model.malformed ? <p className="panel__note">{live.model.malformed} event(s) could not be read and were skipped.</p> : null}
        </FoldSection>
      ) : null}
      <p><a href={labHref('runs')}>Back to all runs</a></p>
      {confirming ? <CancelDialog phase={cancel.phase} onConfirm={onCancelConfirm} onAbort={onCancelAbort} /> : null}
    </RailLayout>
  );
}
