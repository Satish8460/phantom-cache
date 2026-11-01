import React from 'react';
import { TradeoffChart, ShedChart, LatencyChart } from '../../components/LabCharts.jsx';
import Icon from '../../components/Icon.jsx';
import { CellText, FoldSection, LabSection, LabelChip, Stamp, TierBadge, VerdictChip } from '../../components/LabParts.jsx';
import { resultPresentation } from '../../lib/resultPresentation.js';
import { tierInfo } from '../../lib/labProvenance.js';
import { NOT_ESTABLISHED_ALWAYS, NOT_ESTABLISHED_RETAINED, viewAsSummary } from './labCopy.js';

// The result of a Scenario B experiment (retained evidence or a finished run). Order (V2A, DD v2 10.1): Result at a glance
// (the answer, one column per arm, no combined score), Not established (always visible), the binding notice (open), Cause and
// effect, then the second level (paired differences, arms, mechanism, settings) and the limitations (open, never in a closed
// fold). Skill: S16 Simplicity (common path first, detail one level deeper).
//
// U-01: whether verdicts, the mechanism verdict and a VALID-style label may be shown is decided by resultPresentation() from
// the RUN STATE first (lib/resultPresentation.js). A run that did not finish shows "No verdict" and its raw data only inside a
// closed "Partial diagnostic data (not a result)" disclosure; the shape of `verdicts` is never the signal.

/** Entries for the "On this page" list. `p` is the presentation; without verdicts only the answer and the diagnostics exist. */
export function sbContents(view, p) {
  const verdict = !p || p.showVerdicts;
  const items = [{ id: 'sb-glance', label: verdict ? 'Result at a glance' : 'Result' }];
  if (!verdict) {
    items.push({ id: 'sb-partial', label: 'Partial diagnostic data' });
    return items;
  }
  items.push({ id: 'sb-not-established', label: 'Not established' });
  if (view.arms.length) items.push({ id: 'sb-charts', label: 'Cause and effect' }, { id: 'sb-arms', label: 'Arms side by side' });
  if (view.paired.length) items.push({ id: 'sb-paired', label: 'Paired differences' });
  if (view.mechanism.length) items.push({ id: 'sb-mechanism', label: 'Mechanism check' });
  items.push({ id: 'sb-config', label: 'Workload and settings' }, { id: 'sb-limits', label: 'What this does and does not show' });
  return items;
}

function pairedRowOf(view, arm, key) {
  const block = view.paired.find((b) => b.arm === arm);
  return block ? block.rows.find((r) => r.key === key) || null : null;
}

/** One headline quantity of an arm: the paired difference to BASE, its unit, a direction caption, and the 95% interval ONLY when the payload supplied both ends. */
function GlanceFigure({ label, row, unit }) {
  if (!row || !row.present) {
    return (
      <div className="lab-fig" data-glance-figure={label}>
        <dt>{label}</dt>
        <dd><span className="lab-unavailable">Unavailable</span> <span className="lab-hint">(not in the payload)</span></dd>
      </div>
    );
  }
  const hasInterval = row.ciLo.state === 'value' && row.ciHi.state === 'value';
  return (
    <div className="lab-fig" data-glance-figure={label}>
      <dt>{label}</dt>
      <dd className="lab-fig__value"><CellText c={row.diff} /> <span className="lab-fig__unit">{unit}</span></dd>
      <dd className="lab-fig__ci">
        {hasInterval ? <>95% interval <CellText c={row.ciLo} /> to <CellText c={row.ciHi} /></> : <span className="lab-hint">No interval supplied</span>}
        {row.n !== null ? <span className="lab-hint"> · {row.n} pairs</span> : null}
      </dd>
      {row.note ? <dd className="lab-hint">{row.note}</dd> : null}
    </div>
  );
}

function Verdicts({ view, build }) {
  if (!view.verdicts.length) {
    return <p className="panel__note" data-no-verdicts>No arm verdict is recorded in this result. A verdict needs a BASE arm to pair against and at least one capped arm.</p>;
  }
  return (
    <ul className="lab-verdicts lab-glance" aria-label="Verdict per arm, computed by the experiment code">
      {view.verdicts.map((v) => (
        <li key={v.arm} data-verdict-arm={v.arm}>
          <h3 className="lab-glance__arm">{v.arm} <span className="lab-hint">compared with BASE</span></h3>
          <dl className="lab-figs">
            <GlanceFigure label="Peak origin concurrency, change" row={pairedRowOf(view, v.arm, 'origin_peak_concurrency_window')} unit="requests in flight" />
            <GlanceFigure label="Client success rate, change" row={pairedRowOf(view, v.arm, 'client_success_rate_pp')} unit="percentage points" />
          </dl>
          <div className="lab-verdict-head">
            <VerdictChip name={null} outcome={v.outcome} />
            <TierBadge tier="DERIVED" />
          </div>
          <ul className="lab-reasons">
            {v.reasons.map((r) => <li key={r}>{r}</li>)}
          </ul>
          <p className="lab-hint">
            Valid paired runs: {v.validPairs === null ? 'not reported' : v.validPairs}. Mechanism check: {v.mechanismPass === null ? 'not reported' : v.mechanismPass ? 'passed' : 'failed'}.
          </p>
        </li>
      ))}
    </ul>
  );
}

function PairedTable({ block }) {
  return (
    <div className="table-wrap" data-paired-arm={block.arm}>
      <table>
        <caption>{block.arm} compared with BASE, paired by repetition (treatment minus BASE; 95% BCa interval)</caption>
        <thead>
          <tr>
            <th scope="col">Measure</th><th scope="col">BASE mean</th><th scope="col">{block.arm} mean</th><th scope="col">Difference</th>
            <th scope="col">Difference 95% interval</th><th scope="col">Relative change</th><th scope="col">Pairs</th><th scope="col">Provenance</th>
          </tr>
        </thead>
        <tbody>
          {block.rows.map((r) => (r.present ? (
            <tr key={r.key} data-paired-metric={r.key}>
              <th scope="row">{r.label}{r.note ? <span className="lab-hint"> {r.note}</span> : null}</th>
              <td className="num"><CellText c={r.base} /></td>
              <td className="num"><CellText c={r.treat} /></td>
              <td className="num"><CellText c={r.diff} /></td>
              <td className="num"><CellText c={r.ciLo} /> to <CellText c={r.ciHi} /></td>
              <td className="num"><CellText c={r.rel} /></td>
              <td className="num">{r.n === null ? 'not reported' : r.n}</td>
              <td><TierBadge tier="DERIVED" /></td>
            </tr>
          ) : (
            <tr key={r.key} data-paired-metric={r.key}>
              <th scope="row">{r.label}</th>
              <td colSpan={6}><span className="lab-unavailable">Unavailable: not in the payload</span></td>
              <td><TierBadge tier="UNAVAILABLE" /></td>
            </tr>
          )))}
        </tbody>
      </table>
    </div>
  );
}

function ArmsTable({ arms, valueTier, build }) {
  return (
    <div className="table-wrap">
      <table>
        <caption>Per-arm means over repetitions, as reported by the runner</caption>
        <thead>
          <tr>
            <th scope="col">Arm</th><th scope="col">Runs (valid)</th><th scope="col">Peak origin concurrency</th><th scope="col">Peak in degraded phase</th>
            <th scope="col">Client success</th><th scope="col">Shed</th><th scope="col">Lowest cap seen</th><th scope="col">Provenance</th>
          </tr>
        </thead>
        <tbody>
          {arms.map((a) => (
            <tr key={a.name} data-arm-row={a.name}>
              <th scope="row">{a.name}</th>
              <td className="num">{a.runs === null ? 'not reported' : a.runs} ({a.validRuns === null ? 'not reported' : a.validRuns})</td>
              <td className="num"><CellText c={a.peakWindow} /></td>
              <td className="num"><CellText c={a.peakDegraded} /></td>
              <td className="num"><CellText c={a.success} /></td>
              <td className="num"><CellText c={a.shed} /></td>
              <td className="num"><CellText c={a.capMinSeen} /></td>
              <td><TierBadge tier={valueTier} build={build} /></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function SettingsList({ view }) {
  return (
    <dl className="kv" aria-label="Workload and settings">
      {view.config.phases.map((p) => (
        <React.Fragment key={p.name}><dt>{p.name}</dt><dd><CellText c={p.value} /></dd></React.Fragment>
      ))}
      <dt>Arrival rate (req/s)</dt><dd><CellText c={view.config.rate} /></dd>
      <dt>Key catalogue</dt><dd><CellText c={view.config.keys} /></dd>
      <dt>Repetitions</dt><dd><CellText c={view.config.reps} /></dd>
      <dt>Static cap</dt><dd><CellText c={view.config.staticCap} /></dd>
      <dt>AIMD range (min / max)</dt><dd><CellText c={view.config.aimdMin} /> / <CellText c={view.config.aimdMax} /></dd>
      <dt>Origin slowdown factor</dt><dd><CellText c={view.config.degradedMultiplier} /></dd>
    </dl>
  );
}

function Limitations({ view }) {
  return (
    <>
      {view.provenanceNote ? (
        <ul className="lab-reasons">
          {view.provenanceNote.whatRan ? <li>What ran: {view.provenanceNote.whatRan}</li> : null}
          {view.provenanceNote.notCovered ? <li>Not covered: {view.provenanceNote.notCovered}</li> : null}
        </ul>
      ) : null}
      {view.limitations.length ? (
        <>
          <h3 className="lab-subhead">Limitations ({view.limitations.length})</h3>
          <ul className="lab-reasons" data-limitations>{view.limitations.map((l) => <li key={l}>{l}</li>)}</ul>
        </>
      ) : <p className="panel__note" data-limitations-missing>The result carries no limitations list. Do not cite it without the interpretation notice.</p>}
      {view.interpretationNotes.length ? (
        <>
          <h3 className="lab-subhead">Interpretation notes ({view.interpretationNotes.length})</h3>
          <ul className="lab-reasons">{view.interpretationNotes.map((l) => <li key={l}>{l}</li>)}</ul>
        </>
      ) : null}
    </>
  );
}

/**
 * Shown instead of the glance when the rule forbids a verdict (cancelled, failed, interrupted, not computed). The headline and
 * detail come from resultPresentation(); the raw numbers are kept in a CLOSED disclosure and are labelled as not a result.
 */
function NoVerdict({ view, p, valueTier, build }) {
  return (
    <>
      <LabSection id="sb-glance" title="Result" card>
        <div className={`lab-noverdict tone-${p.tone}`} role="status" data-no-verdict data-result-reason={p.reason}>
          <Icon name={p.tone === 'critical' ? 'error' : 'warn'} />
          <div className="lab-noverdict__body">
            <strong className="lab-noverdict__title">{p.headline}</strong>
            {p.detail ? <p>{p.detail}</p> : null}
            {p.showLabel && p.label ? (
              <p data-run-class-incomplete>Run class: <LabelChip label={p.label} /></p>
            ) : null}
            {p.plannedLabel && p.plannedLabel !== p.label ? (
              <p className="lab-hint" data-planned-label>The class planned from the settings was {p.plannedLabel}. It does not apply to a run that did not finish.</p>
            ) : null}
          </div>
        </div>
        {p.labelReasons && p.labelReasons.length ? (
          <ul className="lab-reasons" data-label-reasons>{p.labelReasons.map((r) => <li key={r}>{r}</li>)}</ul>
        ) : null}
      </LabSection>
      {p.rawDiagnostics.available ? (
        <details className="disclosure lab-partial" id="sb-partial" data-partial-diagnostics>
          <summary><span className="lab-choice-group__title">{p.rawDiagnostics.label}</span><span className="lab-choice-group__count">per-arm values, settings, limitations; no verdict</span></summary>
          <div className="lab-partial__body">
            <p className="lab-hint">These values come from a run that did not finish. They show what was recorded before it stopped and support no conclusion about the experiment.</p>
            {view.arms.length ? <ArmsTable arms={view.arms} valueTier={valueTier} build={build} /> : null}
            <h3 className="lab-subhead">Settings</h3>
            <SettingsList view={view} />
            <Limitations view={view} />
          </div>
        </details>
      ) : null}
    </>
  );
}

/**
 * `view`: scenarioBView(). `valueTier`: tier for raw per-arm values; paired differences and verdicts are DERIVED. `build`: 'debug' |
 * 'release' | null, a qualifier on REAL marks. `presentation`: resultPresentation() (computed here from the view when absent, for
 * `state`, default a completed run). `notice`: node rendered after "Not established" (the binding notice). `retained`: true for
 * retained evidence (adds the RETAINED stamp and the retained-only "not established" statements).
 */
export default function ScenarioBResult({ view, valueTier, build, presentation, notice, retained = false, state = 'completed' }) {
  const p = presentation || resultPresentation(state, viewAsSummary(view), { runKind: 'scenario_b' });
  if (!p.showVerdicts) return <NoVerdict view={view} p={p} valueTier={valueTier} build={build} />;
  const noArms = view.arms.length === 0;
  const notEstablished = [...NOT_ESTABLISHED_ALWAYS, ...(retained ? NOT_ESTABLISHED_RETAINED : [])];
  return (
    <>
      <LabSection id="sb-glance" title="Result at a glance" card note={view.verdicts.length > 1 ? 'Each capped arm has its own verdict, computed by the experiment code. This page does not combine them: there is no overall score.' : undefined}>
        <div className="lab-source" data-source-line>
          <TierBadge tier={valueTier} build={build} />
          {retained && !(typeof valueTier === 'object' && valueTier ? valueTier : tierInfo(valueTier)).retained ? <Stamp /> : null}
          {view.status ? <span className="lab-hint">Experiment status as recorded: {view.status}.</span> : null}
        </div>
        <Verdicts view={view} />
        {view.mechanismAllPass !== null ? (
          <p className="lab-hint">Mechanism check (the cap in force was never exceeded at the origin): {view.mechanismAllPass ? 'passed in every arm' : 'failed in at least one arm'}.</p>
        ) : null}
        {view.label ? (
          <div className="lab-runclass" data-runclass>
            <h3 className="lab-subhead">Run class</h3>
            <LabelChip label={view.label} />
            {view.labelMeaning ? <p data-label-meaning>{view.labelMeaning}</p> : null}
          </div>
        ) : null}
        {view.labelReasons.length ? (
          <>
            <h3 className="lab-subhead">Why the label is not VALID</h3>
            <ul className="lab-reasons">{view.labelReasons.map((r) => <li key={r}>{r}</li>)}</ul>
          </>
        ) : null}
      </LabSection>
      <LabSection id="sb-not-established" title="Not established">
        <ul className="lab-reasons lab-notest" data-not-established>
          {view.provenanceNote && view.provenanceNote.notCovered ? <li>{view.provenanceNote.notCovered}</li> : null}
          {notEstablished.map((t) => <li key={t}>{t}</li>)}
        </ul>
        <p className="lab-hint">Read the interpretation notice before citing any number on this page.</p>
      </LabSection>
      {notice || null}
      {noArms ? (
        <p className="panel__note" role="status">This result has no per-arm data, so no charts are drawn.</p>
      ) : (
        <>
          <LabSection id="sb-charts" title="Cause and effect" note="Origin load, shed requests and latency per arm, side by side.">
            <TradeoffChart data={view.charts.tradeoff} tier={valueTier} build={build} />
            <ShedChart shed={view.charts.shed} perPhase={view.charts.shedPerPhase} tier={valueTier} build={build} />
            <LatencyChart latency={view.charts.latency} successOnly={view.charts.latencySuccessOnly} tier={valueTier} build={build} />
          </LabSection>
          <FoldSection id="sb-arms" title="Arms side by side" count={`${view.arms.length} arm(s), values per arm`}>
            <ArmsTable arms={view.arms} valueTier={valueTier} build={build} />
          </FoldSection>
        </>
      )}
      {view.paired.length ? (
        <FoldSection
          id="sb-paired" title="Paired differences with intervals" count={`${view.paired.length} arm(s) compared with BASE`}
          note="Each repetition is paired with the BASE repetition that has the same index. Intervals are 95% BCa bootstrap intervals; p is a sign-flip permutation test. The verdict above applies the experiment's fixed rule to these numbers."
        >
          {view.paired.map((b) => <PairedTable key={b.arm} block={b} />)}
        </FoldSection>
      ) : null}
      {view.mechanism.length ? (
        <FoldSection id="sb-mechanism" title="Mechanism check" count="cap in force versus highest observed" note="Was the cap ever exceeded at the origin? Checked over the whole run including warm-up.">
          <div className="table-wrap">
            <table>
              <caption>Cap in force versus highest origin concurrency observed</caption>
              <thead><tr><th scope="col">Arm</th><th scope="col">Cap in force</th><th scope="col">Highest observed</th><th scope="col">Result</th><th scope="col">Provenance</th></tr></thead>
              <tbody>
                {view.mechanism.map((m) => (
                  <tr key={m.arm}>
                    <th scope="row">{m.arm}</th>
                    <td className="num"><CellText c={m.cap} /></td>
                    <td className="num"><CellText c={m.observed} /></td>
                    <td>{m.pass === null ? 'not reported' : m.pass ? 'passed' : 'failed'}</td>
                    <td><TierBadge tier={valueTier} build={build} /></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </FoldSection>
      ) : null}
      <FoldSection id="sb-config" title="Workload and settings used" count="values recorded by the runner" note="Values recorded by the runner for this experiment.">
        <SettingsList view={view} />
      </FoldSection>
      <LabSection id="sb-limits" title="What this does and does not show" note={`${view.limitations.length} limitation(s), ${view.interpretationNotes.length} interpretation note(s) carried in the result itself. They apply to every citation of these numbers.`}>
        <Limitations view={view} />
      </LabSection>
    </>
  );
}
