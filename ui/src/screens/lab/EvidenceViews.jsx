import React from 'react';
import { StateBlock, SimulationBanner } from '../../components/States.jsx';
import { EffectChart } from '../../components/LabCharts.jsx';
import Icon from '../../components/Icon.jsx';
import { CellText, FoldSection, LabSection, Notice, RailLayout, RichNotice, Stamp, TierBadge, VerdictChip } from '../../components/LabParts.jsx';
import ScenarioBResult, { sbContents } from './ScenarioBResult.jsx';
import { labHref } from '../../lib/routes.js';
import { labelInfo } from '../../lib/labProvenance.js';
import { resultPresentation } from '../../lib/resultPresentation.js';
import { MEASURED_BY_KIND, QUESTION_BY_KIND, viewAsSummary } from './labCopy.js';

// Evidence pages (increment I2): sections with an "On this page" list, the binding notice rendered readably (with a Raw view),
// operational detail in closed folds with counts, charts inside the shared ChartFrame. Cards are used for entities (an experiment
// in the list), never as heading wrappers. Skill: S16 (Simplicity, hierarchy), S15 (prose measure and leading).

// ---------------------------------------------------------------------------------------------------------------
// List

/** One verdict row: icon + word, no border (DD v2 7.4: only provenance and the RETAINED stamp are bordered marks). Negative results use the same size and weight as positive ones. */
function VerdictRow({ chip }) {
  return (
    <li className="lab-vrow" data-verdict={chip.key} data-verdict-kind={chip.kind}>
      <span className="lab-vrow__name">{chip.name}</span>
      <span className={`lab-vrow__word tone-${chip.tone}`}><Icon name={chip.icon} />{chip.label}</span>
    </li>
  );
}

/**
 * Experiment card anatomy (DD v2 8.1), top to bottom: source line (provenance mark + RETAINED stamp), title, the question, verdict
 * rows (one DERIVED mark for the group), one plain sentence for the run class, the action, the meta foot. At most three bordered
 * marks: provenance, stamp, DERIVED. A SIMULATED card sits on a hatched ground; no card shows "Live", moves or animates.
 */
function EvidenceCard({ it, primary = false }) {
  const question = QUESTION_BY_KIND[it.kind.key];
  const cls = labelInfo(it.label);
  const sim = it.tier.src === 'simulated';
  return (
    <li className={`lab-card${sim ? ' hatch' : ''}`} data-evidence-id={it.id} data-evidence-kind={it.kind.key} data-card-kind={sim ? 'sim' : 'real'}>
      <div className="lab-card__source" data-source-line>
        <TierBadge tier={it.tier} />
        {it.exploratory ? <span className="lab-hint" data-exploratory>Exploratory: not the registered evaluation</span> : null}
      </div>
      <h3 className="lab-card__title"><a href={it.href}>{it.kind.label}</a></h3>
      {question ? <p className="lab-card__question" data-card-question>{question}</p> : null}
      {it.verdictChips.length ? (
        <div className="lab-card__verdicts">
          <p className="lab-hint lab-card__verdicts-head">Recorded verdicts <TierBadge tier="DERIVED" /></p>
          <ul className="lab-vrows" aria-label={`Recorded verdicts for ${it.kind.short}`}>
            {it.verdictChips.map((c) => <VerdictRow key={c.name} chip={c} />)}
          </ul>
        </div>
      ) : (
        <p className="panel__note" data-verdict-missing>No verdict recorded: {it.verdictMissing}.</p>
      )}
      {it.label ? <p className="lab-card__class" data-run-class-sentence>{cls.label}.{cls.hint ? ` ${cls.hint}` : ''}</p> : null}
      {it.banner ? <p className="panel__note">{it.banner}</p> : null}
      <p className="lab-card__action">
        <a className={`btn${primary ? ' btn--primary' : ''}`} href={it.href}>Open result</a>
        <span className="lab-hint">{it.artifacts.length} files, listed by name and size; not downloadable here</span>
      </p>
      <p className="lab-card__meta"><span className="mono">{it.id}</span> · Ended {it.ended} · Read-only</p>
    </li>
  );
}

export function EvidenceListBody({ view }) {
  if (!view.items.length) {
    return (
      <StateBlock kind="empty" title="No retained experiments found">
        The Lab API lists retained experiments from its lab directory (PHANTOM_LAB_DIR). None were found there, so there is nothing to show. This is not a result.
      </StateBlock>
    );
  }
  return (
    <>
      <LabSection id="registered" title="Registered experiments" note="Results of experiments that were fixed before they ran. A negative or inconclusive result is a result, and is shown exactly like a positive one.">
        {view.registered.length ? (
          <ul className="lab-cards">{view.registered.map((it, i) => <EvidenceCard key={it.id} it={it} primary={i === 0} />)}</ul>
        ) : <p className="panel__note">None in this directory.</p>}
      </LabSection>
      <LabSection id="exploratory" title="Exploratory runs" note="Dry runs, tuning and smoke runs on development or tuning seeds. They show that the machinery works. They are not evidence for the hypothesis.">
        {view.exploratory.length ? (
          <ul className="lab-cards">{view.exploratory.map((it) => <EvidenceCard key={it.id} it={it} />)}</ul>
        ) : <p className="panel__note">None in this directory.</p>}
      </LabSection>
      <p className="panel__note">{view.total} experiment(s) indexed{view.boundedTo === null ? '' : `; the index covers at most the newest ${view.boundedTo}`}.</p>
    </>
  );
}

// ---------------------------------------------------------------------------------------------------------------
// Detail

const STATIC_M6_LIMITS = [
  'Tier 1 only: a deterministic virtual-time simulation. No real network, host or process was measured.',
  'The stale-window mechanism never bound at the registered horizon, so it was not exercised.',
  'The client P99 endpoint is quantized and saturated in several scenarios (identical values for both arms), so it is a weak discriminator.',
  'One workload set and one cache size. No overhead measurement.',
  'The result does not show that adaptive caching in general is useless; it shows no evidence of benefit for this controller under this protocol.',
];

function M6Detail({ m6, it, sealedNote, provenance }) {
  return (
    <>
      <SimulationBanner>{m6.banner || 'Simulation result. Not a measurement of any network, host or process.'}</SimulationBanner>
      <LabSection id="m6-verdicts" title="Verdicts as recorded" card aside={<TierBadge tier="DERIVED" />} note={m6.ruleSource ? `Rule source: ${m6.ruleSource}` : 'Applied mechanically by the protocol; no judgement after seeing data.'}>
        <ul className="lab-verdicts" aria-label="Hypothesis verdicts">
          {m6.hypotheses.map((h) => (
            <li key={h.id} data-hypothesis={h.id}>
              <VerdictChip name={h.id === 'H5_H6' ? 'H5 and H6' : h.id} outcome={h.verdict} />
              <p><strong>{h.title}</strong></p>
              {h.reason ? <p className="panel__note">{h.reason}</p> : null}
              {h.lines.map((l) => (
                <p key={l.label} className="panel__note">{l.label}: <CellText c={l.value} /> of {l.of} scenarios.</p>
              ))}
              {h.compared ? (
                <p className="panel__note">Cells compared: <CellText c={h.compared.compared} />; identical between arms: <CellText c={h.compared.identical} />.</p>
              ) : null}
              {h.flags.length ? (
                <ul className="lab-reasons" aria-label={`Rule checks for ${h.id}`}>
                  {h.flags.map((f) => <li key={f.key} data-flag={f.key}>{f.label}: <strong>{f.value ? 'yes' : 'no'}</strong></li>)}
                </ul>
              ) : null}
            </li>
          ))}
        </ul>
      </LabSection>
      <LabSection id="m6-effects" title="Effect sizes per scenario" aside={<TierBadge tier={it.tier} />} note="Relative change of the adaptive arm against the comparator named in the protocol (the payload rows do not name it). Positive means the adaptive arm was better; the line at zero is no change. Bars are 95% intervals as recorded.">
        <EffectChart title="Peak origin concurrency" caption="Negative values mean the adaptive arm reached a higher peak than the comparator." chart={m6.peakChart} tier={it.tier} directions />
        <EffectChart title="Client P99 latency" caption="Several scenarios show exactly zero because both arms produced identical P99 values." chart={m6.p99Chart} tier={it.tier} directions />
      </LabSection>
      <FoldSection id="m6-scenarios" title="Per scenario, with constraints" count={`${m6.scenarios.length} scenarios`}>
        <div className="table-wrap">
          <table>
            <caption>Both endpoints and the two constraints for each scenario (values as recorded)</caption>
            <thead>
              <tr>
                <th scope="col">Scenario</th><th scope="col">Peak: comparator mean</th><th scope="col">Peak: adaptive mean</th><th scope="col">Peak: reading</th>
                <th scope="col">P99: reading</th><th scope="col">Fresh-hit change</th><th scope="col">Fresh-hit constraint</th><th scope="col">Error-rate change</th><th scope="col">Error-rate constraint</th>
              </tr>
            </thead>
            <tbody>
              {m6.scenarios.map((s) => (
                <tr key={s.scenario} data-scenario-row={s.scenario}>
                  <th scope="row">{s.scenario}</th>
                  <td className="num">{s.peak ? <CellText c={s.peak.meanCtrl} /> : 'Unavailable'}</td>
                  <td className="num">{s.peak ? <CellText c={s.peak.meanTreat} /> : 'Unavailable'}</td>
                  <td>{s.peak ? (s.peak.supported ? 'supported' : s.peak.adverse ? 'credibly worse' : 'not supported') : 'Unavailable'}</td>
                  <td>{s.p99 ? (s.p99.supported ? 'supported' : s.p99.adverse ? 'credibly worse' : s.p99.practicallyNull ? 'practically null' : 'not supported') : 'Unavailable'}</td>
                  <td className="num"><CellText c={s.freshDelta} /></td>
                  <td>{s.freshOk === null ? 'not reported' : s.freshOk ? 'holds' : 'violated'}</td>
                  <td className="num"><CellText c={s.errorDelta} /></td>
                  <td>{s.errorOk === null ? 'not reported' : s.errorOk ? 'holds' : 'violated'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </FoldSection>
      <LabSection id="m6-falsifiers" title="Falsifiers" note="Pre-registered conditions that would count against the hypothesis.">
        <div className="table-wrap">
          <table>
            <caption>Falsifier status as recorded</caption>
            <thead><tr><th scope="col">Falsifier</th><th scope="col">Status</th><th scope="col">Note</th></tr></thead>
            <tbody>
              {m6.falsifiers.map((f) => (
                <tr key={f.id}><th scope="row">{f.id}</th><td>{f.fires}</td><td>{f.note || ''}</td></tr>
              ))}
            </tbody>
          </table>
        </div>
      </LabSection>
      <LabSection id="m6-limits" title="What this does and does not show">
        <h3 className="lab-subhead">From the result payload</h3>
        <ul className="lab-reasons">
          <li>{m6.banner || 'Tier statement not carried in the payload.'}</li>
          {m6.hypotheses.filter((h) => h.reason).map((h) => <li key={h.id}>{h.id}: {h.reason}</li>)}
          {sealedNote ? <li>{sealedNote}</li> : null}
        </ul>
        <h3 className="lab-subhead">From the research note (static text, not part of the payload)</h3>
        <p className="panel__note">Source: docs/research/m6_sealed_results.md.</p>
        <ul className="lab-reasons" data-static-limits>{STATIC_M6_LIMITS.map((l) => <li key={l}>{l}</li>)}</ul>
      </LabSection>
      <FoldSection id="m6-provenance" title="Provenance" count="protocol, cells, seed set">
        <dl className="kv" aria-label="Provenance">
          <dt>Protocol version</dt><dd>{m6.protocol || 'not reported'}</dd>
          <dt>Cells run / expected</dt><dd><CellText c={m6.cells} /> / <CellText c={m6.expectedCells} /></dd>
          <dt>Completeness problems</dt><dd>{m6.completenessProblems === null ? 'not reported' : m6.completenessProblems}</dd>
          <dt>Seed set</dt><dd>{provenance && provenance.seedSet ? provenance.seedSet : 'not reported'}{sealedNote ? ' (seed values are never shown)' : ''}</dd>
          <dt>Seed list hash (SHA-256)</dt><dd className="mono">{provenance && provenance.seedListSha256 ? provenance.seedListSha256 : 'not reported'}</dd>
          <dt>Arms</dt><dd>{provenance && provenance.arms.length ? provenance.arms.join(', ') : 'not reported'}</dd>
        </dl>
      </FoldSection>
    </>
  );
}

function buildOf(provenance) {
  const p = provenance && provenance.profile;
  return p && (p.driver === 'debug' || p.gateway === 'debug') ? 'debug' : null;
}

function SbDetail({ d }) {
  const notice = d.interpretation;
  const hasNotice = Boolean(notice && notice.noticeText);
  const build = buildOf(d.provenance);
  const noticeSection = hasNotice ? (
    <LabSection
      id="sb-notice" title="Interpretation notice (binding)" card
      note={`${notice.mustAccompany ? 'This text must accompany any citation of these results. ' : ''}${notice.appliesTo || ''}`}
    >
      <RichNotice text={notice.noticeText} label="Binding interpretation notice (the text from the payload)" />
    </LabSection>
  ) : (
    <Notice tone="critical" title="The interpretation notice is missing from this payload" role="alert">
      Do not cite these results without docs/demo/scenario_b_interpretation_notice.md. The Lab API normally carries the notice text; this response did not.
    </Notice>
  );
  return (
    <>
      <Notice tone="info" title="Real, but only on loopback: one host, scripted origin" role="note">
        {d.tier.meaning} This is not a production, WAN or multi-host measurement.
      </Notice>
      {d.sb ? <ScenarioBResult view={d.sb} valueTier={d.tier} build={build} notice={noticeSection} retained /> : noticeSection}
    </>
  );
}

/** Contents entries of an evidence page: the binding notice sits right after "Not established" in Scenario B. */
function evidenceToc(d, isSb, isM6) {
  if (isSb && d.sb) {
    const p = resultPresentation('completed', viewAsSummary(d.sb), { runKind: 'scenario_b' });
    const items = [...sbContents(d.sb, p)];
    const hasNotice = Boolean(d.interpretation && d.interpretation.noticeText);
    if (hasNotice) {
      const at = items.findIndex((i) => i.id === 'sb-not-established');
      items.splice(at >= 0 ? at + 1 : 1, 0, { id: 'sb-notice', label: 'Interpretation notice' });
    }
    return items;
  }
  if (isM6 && d.m6) {
    return [
      { id: 'm6-verdicts', label: 'Verdicts as recorded' }, { id: 'm6-effects', label: 'Effect sizes' }, { id: 'm6-scenarios', label: 'Per scenario' },
      { id: 'm6-falsifiers', label: 'Falsifiers' }, { id: 'm6-limits', label: 'What this does and does not show' }, { id: 'm6-provenance', label: 'Provenance' },
    ];
  }
  return [];
}

export function EvidenceDetailBody({ d }) {
  const isSb = d.kind.key === 'scenario_b';
  const isM6 = d.kind.key === 'm6_sealed_evaluation' || d.kind.key === 'm6_dry_run';
  const build = buildOf(d.provenance);
  const question = QUESTION_BY_KIND[d.kind.key];
  const measured = MEASURED_BY_KIND[d.kind.key];
  return (
    <RailLayout toc={evidenceToc(d, isSb, isM6)}>
      <header className="lab-detail-head lab-evidence-head" data-evidence-detail={d.id}>
        <div className="lab-source" data-source-line>
          <TierBadge tier={d.tier} build={build} />
          {d.tier.retained ? null : <Stamp />}
          {d.originalTier ? <span className="lab-hint">Original tier: {d.originalTier.label}</span> : null}
          {d.exploratory ? <span className="lab-hint">Exploratory</span> : null}
        </div>
        <h2 className="lab-evidence-title" tabIndex={-1}>{d.kind.label}</h2>
        {question ? <p className="lab-question" data-question>{question}</p> : null}
        {measured ? <p className="lab-measured" data-measured><strong>What was measured.</strong> {measured}</p> : null}
        <p className="lab-card__meta"><span className="mono">{d.id}</span> · Started {d.started} · Ended {d.ended} · Read-only: the page displays retained files and never alters them.</p>
      </header>
      {d.resultMissing ? (
        <StateBlock kind="unavailable" title="No result document in this experiment">
          The folder has no readable verdict or summary document, so no numbers can be shown. {d.verdictMissing ? `Reason: ${d.verdictMissing}.` : ''}
        </StateBlock>
      ) : null}
      {d.exploratory ? (
        <Notice tone="warn" title="Exploratory run" role="note">This is a dry run, tuning or smoke run. It shows the machinery works; it is not the registered evaluation.</Notice>
      ) : null}
      {isM6 && d.m6 ? <M6Detail m6={d.m6} it={d} sealedNote={d.sealedNote} provenance={d.provenance} /> : null}
      {isSb ? <SbDetail d={d} /> : null}
      {!isM6 && !isSb && !d.resultMissing ? (
        <Notice tone="info" title="No structured view for this kind" role="status">This page has no specific layout for this kind of experiment. Its files are listed below.</Notice>
      ) : null}
      <FoldSection id="evidence-files" title="Files in this experiment" count={`${d.artifacts.length} file(s)`} note="Names and sizes only; this API does not serve retained files.">
        {d.artifacts.length ? (
          <div className="table-wrap">
            <table>
              <caption>Retained files</caption>
              <thead><tr><th scope="col">Name</th><th scope="col">Size</th></tr></thead>
              <tbody>{d.artifacts.map((a) => <tr key={a.name}><td className="mono">{a.name}</td><td className="num">{a.bytes}</td></tr>)}</tbody>
            </table>
          </div>
        ) : <p className="panel__note">No files listed.</p>}
      </FoldSection>
      {d.provenance ? (
        <FoldSection id="evidence-manifest" title="Manifest provenance" count="tier, scenario, builds, source revision">
          <dl className="kv" aria-label="Manifest provenance">
            <dt>Manifest tier</dt><dd>{d.provenance.manifestTier || 'not reported'}</dd>
            <dt>Scenario</dt><dd>{d.provenance.scenario || 'not reported'}</dd>
            <dt>Status</dt><dd>{d.provenance.status || 'not reported'}</dd>
            <dt>Repetitions</dt><dd>{d.provenance.reps === null ? 'not reported' : d.provenance.reps}</dd>
            <dt>Build profiles</dt><dd>{d.provenance.profile ? `driver ${d.provenance.profile.driver || 'not reported'}, gateway ${d.provenance.profile.gateway || 'not reported'}` : 'not reported'}</dd>
            <dt>Source revision</dt><dd className="mono">{d.provenance.gitHead || 'not reported'}</dd>
          </dl>
        </FoldSection>
      ) : null}
      <p><a href={labHref('evidence')}>Back to all evidence</a></p>
    </RailLayout>
  );
}
