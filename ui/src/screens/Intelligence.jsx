import React from 'react';
import ScreenFrame, { Section } from './ScreenFrame.jsx';
import Placeholder from './Placeholder.jsx';
import ProvenanceBadge from '../components/ProvenanceBadge.jsx';
import { StateBlock, StatusPill } from '../components/States.jsx';
import { useTelemetryContext } from '../hooks/TelemetryContext.js';
import {
  MODE_STATEMENT, NO_BENEFIT_STATEMENT, availability, decisionsView, registryView, signalsView, statusView,
} from '../lib/intelligenceView.js';

// Phantom Intelligence screen (lane L2 step 2). Everything comes from the PIE v2 API (docs/lanes/CONTRACT-pie-api-v1.md); nothing is
// computed or invented here. Each value is followed by the provenance the gateway stated, or by "Unavailable" and the reason.
// When the gateway returns nothing, the page says so. There is no placeholder data and no example row.

/** One value with its provenance and sample count, or the word Unavailable and the reason. */
function Cell({ c, id, hideNote = false }) {
  if (c.text === null) {
    return (
      <>
        <ProvenanceBadge src="unavailable" /> <span className="not-reported__reason">{c.reason}</span>
        {c.samples !== null ? <span className="section__note"> ({c.samples} {c.samples === 1 ? 'sample' : 'samples'})</span> : null}
      </>
    );
  }
  return (
    <>
      <span className="mono" data-intel-value={id}>{c.text}</span>{c.unit ? <span className="section__note"> {c.unit}</span> : null} <ProvenanceBadge src={c.src} />
      {c.samples !== null ? <span className="section__note"> {c.samples.toLocaleString('en-US')} {c.samples === 1 ? 'sample' : 'samples'}</span> : null}
      {c.note && !hideNote ? <span className="section__note"> {c.note}</span> : null}
    </>
  );
}

function ModeBlock({ mode, statement = true }) {
  const unexpected = mode.text !== null && !mode.known;
  return (
    <div data-intel-mode={mode.key}>
      <p>
        Current mode: {mode.text === null ? <Cell c={mode} id="mode" /> : <><strong data-intel-mode-label>{mode.label}</strong> <ProvenanceBadge src={mode.src} /></>}
      </p>
      {unexpected ? (
        <StateBlock kind="warning" title="The gateway reported a mode this page does not recognise">
          It said &ldquo;{mode.label}&rdquo;. Phantom Intelligence has only Off, Observe and Advisory. This is shown as received and is not treated as a normal state.
        </StateBlock>
      ) : null}
      {statement ? <p className="section__note" data-intel-mode-statement>{MODE_STATEMENT}</p> : null}
    </div>
  );
}

function StatusSection({ s }) {
  return (
    <>
      {s.fault ? (
        <StateBlock kind="error" title="Phantom Intelligence stopped itself after a fault">
          Reason {s.fault.reason}{s.fault.detail ? `: ${s.fault.detail}` : ''}. The mode fell to Off and stays Off until the gateway restarts. Serving was not affected.
        </StateBlock>
      ) : null}
      <Section title="Resources and drops" note="What the engine itself costs. Drops are events the observer could not queue; they make a window degraded instead of being invented.">
        <div className="table-wrap">
          <table data-intel-resources>
            <caption>Engine counters</caption>
            <thead><tr><th scope="col">Counter</th><th scope="col">Value</th></tr></thead>
            <tbody>
              <tr><th scope="row">Events queued, total</th><td><Cell c={s.queue.enqueued} id="queue-enqueued" /></td></tr>
              <tr><th scope="row">Events dropped (queue full), total</th><td><Cell c={s.queue.dropped} id="queue-dropped" /></td></tr>
              <tr><th scope="row">Queue depth now</th><td><Cell c={s.queue.depth} id="queue-depth" /></td></tr>
              <tr><th scope="row">Queue capacity (declared)</th><td>{s.queue.capacity.text ?? 'not reported'}</td></tr>
              <tr><th scope="row">Memory used by the engine (upper bound)</th><td><Cell c={s.memory.bytes} id="memory-bytes" hideNote /><span className="section__note"> A sum of declared capacities, not an allocator measurement.</span></td></tr>
              <tr><th scope="row">Memory budget (declared)</th><td>{s.memory.budget ?? 'not reported'}{s.memory.budget ? ' bytes' : ''}</td></tr>
              <tr><th scope="row">Windows closed</th><td><Cell c={s.window.closed} id="windows-closed" /></td></tr>
              <tr><th scope="row">Last window degraded</th><td><Cell c={s.window.lastDegraded} id="window-degraded" /></td></tr>
              <tr><th scope="row">Decisions recorded, total</th><td><Cell c={s.decisions.recorded} id="decisions-recorded" /></td></tr>
              <tr><th scope="row">Decisions kept in the ring</th><td><Cell c={s.decisions.retained} id="decisions-retained" />{s.decisions.ringCapacity ? <span className="section__note"> of {s.decisions.ringCapacity}</span> : null}</td></tr>
              <tr><th scope="row">Decisions suppressed by cooldown</th><td><Cell c={s.decisions.suppressed} id="decisions-suppressed" /></td></tr>
              <tr><th scope="row">Repeated no-action results collapsed</th><td><Cell c={s.decisions.collapsed} id="decisions-collapsed" /></td></tr>
            </tbody>
          </table>
        </div>
      </Section>
      <Section title="Declared configuration and claims" note="Declared by the gateway's configuration, not measured. Shown so a reader can see what the numbers above were produced under.">
        <dl className="kv" aria-label="Declared configuration">
          <dt>Registry version</dt><dd>{s.registryVersion || 'not reported'}</dd>
          <dt>Highest mode allowed</dt><dd>{s.maxMode || 'not reported'}</dd>
          <dt>Lab mode</dt><dd>{s.config.labMode === null ? 'not reported' : s.config.labMode ? 'yes (results may be simulated in a lab run)' : 'no (live gateway)'}</dd>
          <dt>Shadow cache entries</dt><dd>{s.config.cacheCapacity ?? 'not reported'}</dd>
          <dt>Shadow sampling</dt><dd>{s.config.shadowSampleModulus ? `1 key in ${s.config.shadowSampleModulus}` : 'not reported'}</dd>
          <dt>Incumbent eviction policy</dt><dd>{s.config.incumbent || 'not reported'}</dd>
          <dt>Benefit claim</dt><dd data-intel-benefit>{s.claims.benefit || 'not reported'}</dd>
        </dl>
        {s.claims.note ? <p className="section__note">The gateway adds: {s.claims.note}</p> : null}
        <p className="section__note">{NO_BENEFIT_STATEMENT}</p>
      </Section>
    </>
  );
}

function RegistrySection({ reg }) {
  if (!reg.present) return <Section title="Policy registry"><p data-intel-unavailable="registry"><ProvenanceBadge src="unavailable" /> <span className="not-reported__reason">{reg.reason}</span></p></Section>;
  return (
    <Section title="Policy registry" note="The policies the engine knows, their lifecycle stage and the highest mode each may run in. Text fields are declared documentation from the registry, not measurements.">
      <div className="table-wrap">
        <table data-intel-registry>
          <caption>Policies (registry {reg.registryVersion || 'version not reported'}, highest mode {reg.maxMode || 'not reported'})</caption>
          <thead>
            <tr><th scope="col">Policy</th><th scope="col">Kind</th><th scope="col">Lifecycle</th><th scope="col">Highest mode</th><th scope="col">Effective now</th></tr>
          </thead>
          <tbody>
            {reg.policies.map((p) => (
              <tr key={p.id} data-intel-policy={p.id}>
                <th scope="row">{p.id}<br /><span className="section__note">{p.name}{p.implemented ? '' : ' (not implemented)'}</span></th>
                <td>{p.kind}</td>
                <td>{p.lifecycle}</td>
                <td>{p.maxRuntime}</td>
                <td><Cell c={p.effective} id={`policy-${p.id}-effective`} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {reg.truncated ? <p className="section__note">More policies are reported than are listed here.</p> : null}
      {reg.policies.map((p) => (
        <details className="disclosure" key={p.id} data-intel-policy-details={p.id}>
          <summary>{p.id}: declared documentation, inputs and parameters</summary>
          <dl className="kv" aria-label={`${p.id} declared documentation`}>
            {p.docs.map((d) => (<React.Fragment key={d.field}><dt>{d.field.replace(/_/g, ' ')}</dt><dd>{d.text}</dd></React.Fragment>))}
            {p.safety.length ? <><dt>safety constraints</dt><dd>{p.safety.join('; ')}</dd></> : null}
          </dl>
          {p.inputs.length ? (
            <p className="section__note">Needs: {p.inputs.map((i) => `${i.feature} (at least ${i.minSamples ?? '?'} samples, ${i.allowed || 'any provenance'})`).join('; ')}.</p>
          ) : null}
          {p.parameters.length ? (
            <div className="table-wrap">
              <table>
                <caption>{p.id} parameters (declared)</caption>
                <thead><tr><th scope="col">Name</th><th scope="col">Current</th><th scope="col">Default</th><th scope="col">Range</th><th scope="col">Unit</th></tr></thead>
                <tbody>{p.parameters.map((q) => <tr key={q.name}><th scope="row">{q.name}</th><td>{q.current}</td><td>{q.def}</td><td>{q.min} to {q.max}</td><td>{q.unit}</td></tr>)}</tbody>
              </table>
            </div>
          ) : null}
        </details>
      ))}
    </Section>
  );
}

function SignalsSection({ sig }) {
  if (!sig.present) return <Section title="Signals"><p data-intel-unavailable="signals"><ProvenanceBadge src="unavailable" /> <span className="not-reported__reason">{sig.reason}</span></p></Section>;
  return (
    <Section title="Signals" note="Features computed from the last closed window of requests. A feature with too few samples, or from a degraded window, is Unavailable with the reason; values are never extrapolated.">
      {sig.window ? (
        <p data-intel-window>
          Window {sig.window.index ?? '?'}: {sig.window.durationMs} ms, {sig.window.events} events, {sig.window.dropped} dropped.
          {sig.window.degraded ? <> <StatusPill tone="warn" icon="warn">Degraded{sig.window.degradedReason ? `: ${sig.window.degradedReason.replace(/_/g, ' ')}` : ''}</StatusPill></> : null}
        </p>
      ) : <p data-intel-window="none">No window has closed yet, so no feature has a value.</p>}
      <div className="table-wrap">
        <table data-intel-features>
          <caption>Features of the last window</caption>
          <thead><tr><th scope="col">Feature</th><th scope="col">Value</th></tr></thead>
          <tbody>
            {sig.features.length === 0 ? <tr><td colSpan={2}>The gateway reported no features.</td></tr> : sig.features.map((f) => (
              <tr key={f.id} data-intel-feature={f.id}>
                <th scope="row">{f.id} {f.name}</th>
                <td><Cell c={f.cell} id={`feature-${f.id}`} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <p>Workload class (policy P1): <Cell c={sig.classification} id="classification" /></p>
      {sig.topKeys.items.length ? (
        <div className="table-wrap">
          <table data-intel-topkeys>
            <caption>Most requested keys (hashes only; no key or tenant is ever sent) <ProvenanceBadge src={sig.topKeys.src} /></caption>
            <thead><tr><th scope="col">Key hash</th><th scope="col">Requests in window</th></tr></thead>
            <tbody>{sig.topKeys.items.map((k) => <tr key={k.hash}><th scope="row" className="mono">{k.hash}</th><td>{k.count}</td></tr>)}</tbody>
          </table>
        </div>
      ) : null}
      {sig.history.length ? <p className="section__note">Last {sig.history.length} windows (oldest first): {sig.history.map((h) => `#${h.index ?? '?'} ${h.events} events${h.degraded ? ' (degraded)' : ''}`).join('; ')}.</p> : null}
    </Section>
  );
}

function DecisionRecord({ r, open }) {
  return (
    <details className="disclosure" open={open} data-intel-decision={r.id} data-intel-verdict={r.verdict}>
      <summary>#{r.id ?? '?'} {r.policyId} {r.verdict} ({r.reasonCode || 'no reason code'}) {r.modeKnown ? null : <StatusPill tone="warn" icon="warn">unexpected mode</StatusPill>}</summary>
      <p className="section__note">At {r.timestampMs} ms on the gateway&rsquo;s process clock (not wall time). Mode when decided: {r.mode}. Policy version {r.policyVersion}.</p>
      {r.simulatedClaim ? <StateBlock kind="warning" title="This record says it is simulated">A live gateway does not produce simulated results. It is listed, but this page does not treat it as a live decision.</StateBlock> : null}
      {r.recommendation ? (
        <p data-intel-recommendation><StatusPill tone="info" icon="info">{r.recommendation.label}</StatusPill> {r.recommendation.text} <strong>Not applied: nothing was changed.</strong></p>
      ) : null}
      {r.recommendationWithheld ? <StateBlock kind="warning" title="A recommendation was withheld">The record carries a recommendation that does not state it was not applied. This page shows only advice that is marked not applied.</StateBlock> : null}
      <p data-intel-explanation>{r.explanation || 'No explanation text in this record.'}</p>
      {r.inputs.length ? (
        <div className="table-wrap">
          <table>
            <caption>Inputs the policy used</caption>
            <thead><tr><th scope="col">Feature</th><th scope="col">Value</th><th scope="col">Window</th></tr></thead>
            <tbody>{r.inputs.map((i, k) => <tr key={`${i.feature}-${k}`}><th scope="row">{i.feature}</th><td><Cell c={i.cell} id={`decision-${r.id}-in-${i.feature}`} /></td><td>{i.windowMs} ms</td></tr>)}</tbody>
          </table>
        </div>
      ) : <p className="section__note">No inputs are listed for this record.</p>}
      {r.candidates.length ? (
        <div className="table-wrap">
          <table>
            <caption>Candidates considered, with the estimated effect where the policy produced one</caption>
            <thead><tr><th scope="col">Candidate</th><th scope="col">Outcome</th><th scope="col">Estimated effect (shadow estimate)</th></tr></thead>
            <tbody>
              {r.candidates.map((c) => (
                <tr key={c.id} data-intel-candidate={c.id}>
                  <th scope="row">{c.id}</th>
                  <td>{c.status}{c.rejection ? `: ${c.rejection.replace(/_/g, ' ')}` : ''}</td>
                  <td>
                    {c.effect && c.effect.cell ? <><Cell c={c.effect.cell} id={`decision-${r.id}-effect-${c.id}`} /> {c.effect.metric}{c.effect.ci95 ? `, 95% interval ${c.effect.ci95}` : ''}{c.effect.basis ? ` (${c.effect.basis.replace(/_/g, ' ')})` : ''}</> : 'none estimated'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
      {r.observed.text !== null ? <p data-intel-observed>Observed effect after the fact: <Cell c={r.observed} id={`decision-${r.id}-observed`} /></p> : null}
      {r.constraints.length ? <p className="section__note">Constraints: {r.constraints.map((c) => `${c.name} ${c.value} (${c.holds ? 'holds' : 'DOES NOT HOLD'})`).join('; ')}.</p> : null}
      {r.guardrails.length ? <p className="section__note">Guardrail events: {r.guardrails.join('; ')}.</p> : null}
      {r.conflict ? <p className="section__note">Conflict on {r.conflict.lever.replace(/_/g, ' ')}: this policy is {r.conflict.role}{r.conflict.with.length ? `, with decisions ${r.conflict.with.map((x) => `#${x}`).join(', ')}` : ''}.</p> : null}
    </details>
  );
}

function DecisionsSection({ dec }) {
  if (!dec.present) return <Section title="Recent decisions"><p data-intel-unavailable="decisions"><ProvenanceBadge src="unavailable" /> <span className="not-reported__reason">{dec.reason}</span></p></Section>;
  return (
    <Section title="Recent decisions" note={`Newest first. ${dec.totalRecorded ?? '?'} recorded since start, ${dec.retained ?? '?'} kept. Every recommendation is advice that was not applied.`}>
      {dec.records.length === 0 ? <p data-intel-decisions="none">No decisions have been recorded. In Observe mode only faults are recorded; in Advisory mode a record appears when a policy reaches a verdict that differs from the last one.</p> : null}
      {dec.records.map((r, i) => <DecisionRecord key={`${r.id}-${i}`} r={r} open={i === 0 || r.verdict === 'Recommend'} />)}
      {dec.ordering === 'unexpected' ? <p className="section__note">The gateway did not state the record order as oldest first; the order shown may be wrong.</p> : null}
    </Section>
  );
}

/**
 * The screen body, from the raw API answers (see hooks/useIntelligence.js). Presentational: the harness renders it with fixtures.
 */
export function IntelligenceBody({ raw, onRefresh }) {
  const av = availability(raw.status);
  const status = statusView(raw.status);
  if (av === 'loading') {
    return <StateBlock kind="loading" title="Asking the gateway about Phantom Intelligence">No values are shown until the gateway answers.</StateBlock>;
  }
  if (av === 'unauthorised') {
    return <StateBlock kind="warning" title="The gateway refused the request">The intelligence endpoints need a token with the telemetry:read role on a secure gateway. The sign-in panel at the top of the page says what the gateway answered and lets you paste a token. No values are shown.</StateBlock>;
  }
  if (av === 'no_answer') {
    return <StateBlock kind="disconnected" title="No answer from the gateway">The request for the intelligence status did not reach the gateway. No values are shown.</StateBlock>;
  }
  if (av === 'endpoints_missing') {
    return (
      <>
        <StateBlock kind="notEnabled" title="This gateway does not serve the Phantom Intelligence API">It answered 404 for the status endpoint. It is probably an older build. Nothing is shown here because nothing was reported.</StateBlock>
        <Placeholder screenId="intelligence" />
      </>
    );
  }
  if (av === 'error') {
    return <StateBlock kind="error" title="The gateway answered, but not in a form this page can read">The status answer is not in the pie-api-v1 shape. The page and the gateway may be different versions. No values are shown.</StateBlock>;
  }
  const refresh = onRefresh ? <p><button type="button" className="btn" onClick={onRefresh}>Reload registry now</button></p> : null;
  if (av === 'legacy') {
    return (
      <>
        <Section title="Mode"><ModeBlock mode={status.mode} /></Section>
        <StateBlock kind="notEnabled" title="This gateway reports only the older intelligence status">
          It does not serve the registry, signals or decisions endpoints, so there is no policy list, no window and no decision record to show. {status.legacy && status.legacy.truthful === false ? 'It also states that its intelligence output is not truthful (simulation enabled).' : ''}
        </StateBlock>
      </>
    );
  }
  if (av === 'not_installed') {
    return (
      <>
        <Section title="Mode"><ModeBlock mode={status.mode} /></Section>
        <StateBlock kind="notEnabled" title="Phantom Intelligence v2 is not installed in this gateway process">{status.reason}. Start the gateway with the intelligence engine installed to see the registry, signals and decisions. Nothing is shown here because nothing is running.</StateBlock>
      </>
    );
  }
  const reg = registryView(raw.registry);
  const sig = signalsView(raw.signals);
  const dec = decisionsView(raw.decisions);
  return (
    <>
      <Section title="Mode" note="What the engine is allowed to do in this gateway process right now.">
        <ModeBlock mode={status.mode} />
        <p className="section__note" data-intel-applied>Applied changes: none. This engine never changes how the gateway serves requests; hard caps (staleness, origin load, memory) and the fencing rules are enforced by the serving layer whatever the engine says.</p>
      </Section>
      <StatusSection s={status} />
      <RegistrySection reg={reg} />
      <SignalsSection sig={sig} />
      <DecisionsSection dec={dec} />
      {refresh}
    </>
  );
}

export default function Intelligence() {
  const ctx = useTelemetryContext();
  const raw = ctx.intel ? ctx.intel.raw : { status: null, registry: null, signals: null, decisions: null };
  return (
    <ScreenFrame
      screenId="intelligence"
      group={availability(raw.status) === 'ready' ? 'Live gateway' : undefined}
      title="Phantom Intelligence"
      intro="What the policy engine observes, which policies exist, and the decisions it recorded, each with the inputs and the candidates it rejected. Observe and Advisory only: nothing here is applied to the gateway."
    >
      <IntelligenceBody raw={raw} onRefresh={ctx.intel ? ctx.intel.refresh : undefined} />
    </ScreenFrame>
  );
}
