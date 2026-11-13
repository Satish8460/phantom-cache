import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { StateBlock } from '../../components/States.jsx';
import { useTelemetryContext } from '../../hooks/TelemetryContext.js';
import { LabContext, useLab } from '../../hooks/LabContext.js';
import { useLabResource } from '../../hooks/useLabResource.js';
import { useRunStream } from '../../hooks/useRunStream.js';
import { createLabClient } from '../../lib/labApi.js';
import { tokenRoles } from '../../lib/labToken.js';
import { labHref } from '../../lib/routes.js';
import { resultPresentation } from '../../lib/resultPresentation.js';
import { artifactsView, capabilitiesView, evidenceDetailView, evidenceListView, runView, runsListView, scenarioBView } from '../../lib/labView.js';
import ConnectionPanel from './ConnectionPanel.jsx';
import { ErrorBlock, ReadForbiddenBlock, ResourceGate } from './Gate.jsx';
import StoryView from './StoryView.jsx';
import { EvidenceDetailBody, EvidenceListBody } from './EvidenceViews.jsx';
import { RunFormBody } from './RunForm.jsx';
import { RunDetailBody, RunsListBody } from './RunsViews.jsx';

// ---------------------------------------------------------------------------------------------------------------
// Provider: one client per (gateway, token). The token lives in a ref (memory only). Changing it creates a new
// client object, which reloads every resource and restarts any open stream with the new credential.

function LabProvider({ baseUrl, children }) {
  const tokenRef = useRef(null);
  const [version, setVersion] = useState(0);
  const [roles, setRoles] = useState(null); // role names read from the token for display only (null = unknown)
  const client = useMemo(
    () => createLabClient({ baseUrl, getToken: () => tokenRef.current }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [baseUrl, version],
  );
  const setToken = useCallback((t) => {
    tokenRef.current = t || null;
    setRoles(t ? tokenRoles(t) : null);
    setVersion((v) => v + 1);
  }, []);
  const clearToken = useCallback(() => {
    tokenRef.current = null;
    setRoles(null);
    setVersion((v) => v + 1);
  }, []);
  const caps = useLabResource((signal) => client.capabilities({ signal }), [client]);
  const capsView = useMemo(() => (caps.data ? capabilitiesView(caps.data) : null), [caps.data]);
  const value = useMemo(
    () => ({ client, baseUrl, hasToken: tokenRef.current !== null, roles, setToken, clearToken, caps, capsView, reloadCaps: caps.reload }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [client, baseUrl, version, roles, setToken, clearToken, caps, capsView],
  );
  return <LabContext.Provider value={value}>{children}</LabContext.Provider>;
}

// Workflow order (increment I2): what the Lab is, then the main task (start a run), then its results, then retained evidence.
// Ids and hashes are the existing routes; only the visible order and the first label changed ("Overview" clashed with the
// app-level Overview page). routes.js (increment I4) still lists them in the old order.
const LAB_TABS = Object.freeze([
  { id: 'story', label: 'About' },
  { id: 'run', label: 'Start a run' },
  { id: 'runs', label: 'Runs' },
  { id: 'evidence', label: 'Evidence' },
]);

function LabNav({ current }) {
  return (
    <nav className="lab-subnav" aria-label="Lab sections">
      <ul className="tabs">
        {LAB_TABS.map((v) => (
          <li key={v.id}>
            <a className="tab" href={labHref(v.id)} aria-current={v.id === current ? 'page' : undefined} data-lab-view={v.id}>{v.label}</a>
          </li>
        ))}
      </ul>
    </nav>
  );
}

// ---------------------------------------------------------------------------------------------------------------
// Pages (containers: hooks here, presentation in the *Body components)

function StoryPage() {
  const { client } = useLab();
  const idx = useLabResource((signal) => client.evidenceIndex({ signal }), [client]);
  const view = useMemo(() => (idx.data ? evidenceListView(idx.data) : null), [idx.data]);
  const m6Id = view ? view.items.find((i) => i.kind.key === 'm6_sealed_evaluation')?.id : null;
  const sbId = view ? view.items.find((i) => i.kind.key === 'scenario_b')?.id : null;
  const m6 = useLabResource((signal) => client.evidence(m6Id, { signal }), [client, m6Id], { enabled: Boolean(m6Id) });
  const sb = useLabResource((signal) => client.evidence(sbId, { signal }), [client, sbId], { enabled: Boolean(sbId) });
  const m6View = useMemo(() => (m6.data ? evidenceDetailView(m6.data) : null), [m6.data]);
  const sbView = useMemo(() => (sb.data ? evidenceDetailView(sb.data) : null), [sb.data]);
  return (
    <ResourceGate res={idx} what="the evidence index">
      {() => (view ? <StoryView evidence={view} m6Detail={m6View ? m6View.m6 : null} sbDetail={sbView ? sbView.sb : null} /> : <ErrorBlock error={{ kind: 'unexpected', title: 'The evidence index was not in the expected format', fix: '' }} />)}
    </ResourceGate>
  );
}

function EvidenceListPage() {
  const { client } = useLab();
  const res = useLabResource((signal) => client.evidenceIndex({ signal }), [client]);
  return (
    <ResourceGate res={res} what="the evidence index">
      {(data) => {
        const v = evidenceListView(data);
        return v ? <EvidenceListBody view={v} /> : <ErrorBlock error={{ kind: 'unexpected', title: 'The evidence index was not in the expected format', fix: '' }} />;
      }}
    </ResourceGate>
  );
}

function EvidenceDetailPage({ id }) {
  const { client } = useLab();
  const res = useLabResource((signal) => client.evidence(id, { signal }), [client, id]);
  return (
    <ResourceGate res={res} what="this result">
      {(data) => {
        const v = evidenceDetailView(data);
        return v ? <EvidenceDetailBody d={v} /> : <ErrorBlock error={{ kind: 'unexpected', title: 'The result was not in the expected format', fix: '' }} />;
      }}
    </ResourceGate>
  );
}

function RunPage() {
  const { client, caps, capsView, roles } = useLab();
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState(null);
  const onSubmit = async (kind, spec) => {
    setSubmitting(true);
    setSubmitError(null);
    const r = await client.startRun(kind, spec);
    setSubmitting(false);
    if (r.ok && typeof r.data.run_id === 'string') {
      window.location.hash = labHref('runs', r.data.run_id);
    } else if (r.ok) {
      setSubmitError({ title: 'The gateway accepted the run but returned no run id', fix: 'Open the Runs list to find it.', serverMessage: '' });
    } else {
      setSubmitError(r.error);
    }
  };
  return (
    <ResourceGate res={caps} what="the run limits">
      {() => (capsView ? <RunFormBody caps={capsView} roles={roles} onSubmit={onSubmit} submitting={submitting} submitError={submitError} /> : null)}
    </ResourceGate>
  );
}

function RunsPage() {
  const { client } = useLab();
  const res = useLabResource((signal) => client.runs({ signal }), [client]);
  return (
    <ResourceGate res={res} what="the run history">
      {(data) => {
        const v = runsListView(data);
        return v ? <RunsListBody view={v} onRefresh={res.reload} /> : <ErrorBlock error={{ kind: 'unexpected', title: 'The run list was not in the expected format', fix: '' }} />;
      }}
    </ResourceGate>
  );
}

function triggerDownload(blob, name) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  a.rel = 'noopener';
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10000);
}

function RunDetailPage({ id }) {
  const { client, roles } = useLab();
  const runRes = useLabResource((signal) => client.run(id, { signal }), [client, id]);
  const rv = useMemo(() => (runRes.data ? runView(runRes.data) : null), [runRes.data]);
  const live = useRunStream(client, id, { enabled: Boolean(runRes.data) });
  const done = live.model.done ? live.model.done.state : null;
  const effectiveState = live.model.state || (rv ? rv.state : null);
  const terminal = ['completed', 'failed', 'cancelled', 'interrupted'].includes(effectiveState);
  const reloadRun = runRes.reload;

  // When the stream says the run is over, re-read the persisted record once (exit code, artifact index).
  useEffect(() => {
    if (done) reloadRun();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [done]);

  const artRes = useLabResource((signal) => client.artifacts(id, { signal }), [client, id, terminal, done], { enabled: Boolean(runRes.data) });
  const artRows = useMemo(() => (artRes.data ? artifactsView(artRes.data) : null), [artRes.data]);
  // U-01: whether summary.json is read at all is part of the rule (a failed or interrupted run never auto-fetches it, D-13).
  const progressNow = live.model.progress || (rv && rv.progress.total !== null ? { done: rv.progress.done, total: rv.progress.total } : null);
  const rule0 = rv ? resultPresentation(effectiveState, null, { runKind: rv.kind.key, progress: progressNow }) : null;
  const wantSummary = Boolean(rv && rv.kind.key === 'scenario_b' && terminal && rule0 && rule0.fetchSummary && (rv.hasSummaryArtifact || (artRows && artRows.some((a) => a.name === 'summary.json'))));
  const sumRes = useLabResource(async (signal) => {
    const r = await client.artifactJson(id, 'summary.json', { signal });
    if (!r.ok) return r;
    const v = scenarioBView(r.data);
    // The raw JSON travels with the view: resultPresentation() reads status, completeness and verdict signals from it.
    return v ? { ok: true, data: { view: v, raw: r.data } } : { ok: false, error: { kind: 'unexpected', title: 'summary.json was not in the expected format', fix: '', serverMessage: '' } };
  }, [client, id, wantSummary, done], { enabled: wantSummary });

  const [cancel, setCancel] = useState({ phase: 'idle', error: null });
  const [downloading, setDownloading] = useState(null);
  const [downloadError, setDownloadError] = useState(null);
  const onCancelConfirm = async () => {
    setCancel({ phase: 'sending', error: null });
    const r = await client.cancelRun(id);
    if (r.ok) {
      setCancel({ phase: 'sent', error: null });
      reloadRun();
    } else {
      setCancel({ phase: 'error', error: r.error });
    }
  };
  const onDownload = async (name) => {
    setDownloading(name);
    setDownloadError(null);
    const r = await client.downloadArtifact(id, name);
    setDownloading(null);
    if (r.ok) {
      try {
        triggerDownload(r.blob, name);
      } catch {
        setDownloadError({ kind: 'unexpected', title: 'The browser could not save the file', fix: 'Try again or use the phantom-lab command line.', serverMessage: '' });
      }
    } else {
      setDownloadError(r.error);
    }
  };

  if (runRes.status === 'loading') return <StateBlock kind="loading" title="Loading the run">Waiting for the gateway to answer.</StateBlock>;
  if (runRes.status === 'error' && !rv && runRes.error && runRes.error.kind === 'forbidden_role') return <ReadForbiddenBlock error={runRes.error} runId={id} roles={roles} onRetry={runRes.reload} />;
  if (runRes.status === 'error' && !rv) return <ErrorBlock error={runRes.error} onRetry={runRes.reload} what="this run" />;
  if (!rv) return <ErrorBlock error={{ kind: 'unexpected', title: 'The run was not in the expected format', fix: '' }} />;

  const artifacts = artRes.status === 'ready' && artRows
    ? { state: 'ready', rows: artRows }
    : artRes.status === 'error' ? { state: 'error', error: artRes.error } : { state: 'loading', rows: [] };
  const summary = sumRes.status === 'ready' ? { state: 'ready', view: sumRes.data.view, raw: sumRes.data.raw }
    : sumRes.status === 'error' ? { state: 'error', error: sumRes.error }
      : wantSummary ? { state: 'loading' } : { state: 'none' };
  return (
    <RunDetailBody
      run={rv}
      live={live}
      artifacts={artifacts}
      summary={summary}
      cancel={cancel}
      onCancelStart={() => setCancel({ phase: 'confirm', error: null })}
      onCancelConfirm={onCancelConfirm}
      onCancelAbort={() => setCancel({ phase: 'idle', error: null })}
      onReconnect={live.reconnectNow}
      onRefresh={reloadRun}
      onDownload={onDownload}
      downloading={downloading}
      downloadError={downloadError}
    />
  );
}

const SUBVIEW_TITLE = Object.freeze({
  story: 'About the experiments: problem, hypothesis, evidence',
  evidence: 'Evidence',
  run: 'Start a run',
  runs: 'Runs',
});

function Body({ sub }) {
  const view = sub ? sub.view : 'story';
  switch (view) {
    case 'evidence': return sub.id ? <EvidenceDetailPage key={sub.id} id={sub.id} /> : <EvidenceListPage />;
    case 'run': return <RunPage />;
    case 'runs': return sub.id ? <RunDetailPage key={sub.id} id={sub.id} /> : <RunsPage />;
    default: return <StoryPage />;
  }
}

function LabInner({ sub, gatewayUrl }) {
  const { caps, capsView, hasToken, roles, setToken, clearToken } = useLab();
  const view = sub ? sub.view : 'story';
  const disabled = caps.status === 'error' && caps.error && caps.error.kind === 'disabled';
  return (
    <>
      <div className="page-head lab-head">
        <div className="lab-head__row">
          <h1 tabIndex={-1} data-screen-heading>Phantom Lab</h1>
          {view !== 'run' && !disabled ? <a className="btn btn--primary lab-head__action" href={labHref('run')} data-primary-action>Start a run</a> : null}
        </div>
        <p>Experiments on origin protection: what was tested, what it showed, and how to run it again. Every number carries the label of where it came from.</p>
      </div>
      <ConnectionPanel res={caps} view={capsView} hasToken={hasToken} roles={roles} onToken={setToken} onClear={clearToken} gatewayUrl={gatewayUrl} />
      {disabled ? (
        <StateBlock kind="notEnabled" title="The Lab is switched off">
          The sections below need the Lab API. Enable it on the gateway and reload this page. No retained result and no run history is shown while it is off, and nothing is shown in their place.
        </StateBlock>
      ) : (
        <>
          <LabNav current={view} />
          <section aria-label={SUBVIEW_TITLE[view] || 'Lab'} className="lab-body" data-lab-body={view}>
            <Body sub={sub} />
          </section>
        </>
      )}
    </>
  );
}

/** The Phantom Lab screen. `sub` is the parsed sub-route ({view, id}) or undefined for the overview. */
export default function LabScreen({ sub }) {
  const { gateway } = useTelemetryContext();
  return (
    <LabProvider baseUrl={gateway.url}>
      <LabInner sub={sub} gatewayUrl={gateway.url} />
    </LabProvider>
  );
}
