import React from 'react';
import { StateBlock } from '../../components/States.jsx';
import { CellText, LabSection, LabelChip, Notice, TierBadge, VerdictChip } from '../../components/LabParts.jsx';
import { TIERS } from '../../lib/labProvenance.js';

function pickLatest(view, kind) {
  return view.items.find((i) => i.kind.key === kind) || null;
}

function ResultCard({ title, item, children }) {
  return (
    <section className="lab-card" aria-label={title} data-story-card={item ? item.kind.key : 'none'}>
      <div className="lab-card__head">
        <h3>{title}</h3>
        {item ? <TierBadge tier={item.tier} /> : null}
        {item && item.label ? <LabelChip label={item.label} /> : null}
      </div>
      {item ? (
        <>
          <ul className="lab-chips" aria-label={`Recorded verdicts: ${title}`}>
            {item.verdictChips.map((c) => <li key={c.name}><VerdictChip name={c.name} outcome={c} /></li>)}
          </ul>
          {children}
          <p><a href={item.href}>Open the full result</a></p>
        </>
      ) : (
        <p className="panel__note">No retained result of this kind was found in the lab directory. Nothing is shown in its place.</p>
      )}
    </section>
  );
}

/**
 * The story: problem, hypothesis, what the evidence shows. Facts come from the evidence index (and the two
 * detail documents, when they loaded); everything else is explanation in words with no numbers.
 */
export default function StoryView({ evidence, m6Detail, sbDetail }) {
  const m6 = evidence ? pickLatest(evidence, 'm6_sealed_evaluation') : null;
  const sb = evidence ? pickLatest(evidence, 'scenario_b') : null;
  const h1 = m6Detail ? m6Detail.hypotheses.find((h) => h.id === 'H1') : null;
  // Sentences below are chosen from the recorded verdict kinds; if the kinds are not these, nothing is asserted.
  const readings = [];
  if (m6 && m6.verdictChips.some((c) => c.name === 'H1' && c.kind === 'negative')) {
    readings.push('The sealed simulation recorded a negative result for H1: the registered hypothesis was not supported.');
  }
  if (sb && sb.verdictChips.some((c) => c.key === 'TRADE_OFF_FAILURE')) {
    readings.push('Scenario B recorded a trade-off failure: a cap lowered peak load at the origin, and clients paid for it with shed requests. The reasons above state the amounts.');
  }
  return (
    <>
      <LabSection id="problem" title="The problem">
        <div className="prose">
          <p>
            PhantomCache sits between clients and an origin server and serves repeated requests from a cache. When the origin gets slow, requests pile up at it. A cap on how many requests may be in flight at the origin protects it, but requests over the cap have to be turned away.
          </p>
          <p>The open question is whether a controller that adjusts that cap on its own can protect the origin better than a fixed cap, without turning away more clients.</p>
        </div>
      </LabSection>
      <LabSection id="hypothesis" title="The hypothesis" note="Registered before the evaluation ran (docs/research/m6_sealed_results.md).">
        <p className="prose">
          <strong>H1:</strong> an origin-aware adaptive concurrency controller beats the best static configuration. The evaluation was fixed in advance: seeds, scenarios, endpoints and the rule that turns numbers into a verdict. The rule was applied mechanically.
        </p>
      </LabSection>
      <LabSection id="evidence" title="What the evidence shows" note="Read from the Lab API. Both results were produced by a fixed rule and are shown with the same weight.">
        {evidence ? (
          <div className="lab-cards-row">
            <ResultCard title="Simulation, sealed seeds (M6)" item={m6}>
              {m6 && h1 && h1.lines.length === 2 ? (
                <p data-story-m6-counts>
                  Peak origin concurrency was supported in <CellText c={h1.lines[0].value} /> of {h1.lines[0].of} scenarios.
                  Client P99 was supported in <CellText c={h1.lines[1].value} /> of {h1.lines[1].of} scenarios.
                </p>
              ) : m6 ? <p className="panel__note">The detailed counts did not load; open the full result.</p> : null}
              {m6 && m6.banner ? <p className="panel__note">{m6.banner}</p> : null}
            </ResultCard>
            <ResultCard title="Real, on loopback: Scenario B" item={sb}>
              {sb && sbDetail ? (
                <ul className="lab-reasons" data-story-sb-reasons>
                  {sbDetail.verdicts.flatMap((v) => v.reasons.map((r) => <li key={`${v.arm}-${r}`}><strong>{v.arm}:</strong> {r}</li>))}
                </ul>
              ) : sb ? <p className="panel__note">The detailed reasons did not load; open the full result.</p> : null}
              {sb && sb.label ? <p className="panel__note">The VALID label has a narrow meaning stated on the result page: release builds, enough repetitions, one host, scripted origin. It does not mean the controller works.</p> : null}
            </ResultCard>
          </div>
        ) : null}
        {evidence && evidence.items.length === 0 ? (
          <StateBlock kind="empty" title="No retained results to summarise">The lab directory holds no retained experiments, so the story has no evidence section.</StateBlock>
        ) : null}
        {readings.length ? (
          <Notice tone="info" title="How to read the results together" role="note">
            <ul className="lab-reasons" data-story-readings>{readings.map((r) => <li key={r}>{r}</li>)}</ul>
          </Notice>
        ) : null}
      </LabSection>
      <LabSection id="not-shown" title="What this does not show">
        <ul className="lab-reasons">
          <li>That adaptive control is useless in general. One controller design, one objective and two stand-in origins were tested.</li>
          <li>Anything about production, wide-area networks or multi-host deployments.</li>
          <li>That latency improved for clients who were served: all-request percentiles include fast-failing rejections.</li>
          <li>That stale serving works under overload: the Scenario B workload never exercised it.</li>
          <li>That the cap values were chosen in advance. They were picked after a pilot run.</li>
        </ul>
        <Notice tone="info" title="The intelligence engine is not part of the live serving path">
          Phantom Intelligence (observe, detect, predict, evaluate, adapt, explain) is a research track. According to the project accountability report it is not integrated into request serving; only the origin-concurrency gate is wired, and it is enforced only when the gateway runs in canary or active mode. This page displays experiments. It does not control traffic.
        </Notice>
      </LabSection>
      <LabSection id="labels" title="How to read the labels" note="Every number in the Lab carries one of these. The badge says where the number came from.">
        <ul className="lab-legend" aria-label="Provenance labels">
          {['MEASURED_LOOPBACK', 'SIMULATED', 'DERIVED', 'UNAVAILABLE'].map((k) => (
            <li key={k}><TierBadge tier={k} /> <span>{TIERS[k].meaning}</span></li>
          ))}
          <li><span className="tag" data-retained-legend>Retained</span> <span>A separate tag, not a kind of number: the experiment was read from an earlier folder and this page only displays it. The badge next to it still says whether the number is real or simulated.</span></li>
        </ul>
      </LabSection>
    </>
  );
}
