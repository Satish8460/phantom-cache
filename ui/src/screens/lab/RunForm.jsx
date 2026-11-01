import React, { useEffect, useId, useMemo, useRef, useState } from 'react';
import { Notice, TierBadge, CopyButton } from '../../components/LabParts.jsx';
import { DisabledReason, StateBlock } from '../../components/States.jsx';
import Icon from '../../components/Icon.jsx';
import ProvenanceBadge from '../../components/ProvenanceBadge.jsx';
import { plannedMinimum } from '../../lib/progress.js';
import { roleCapabilities } from '../../lib/labToken.js';
import {
  ADVANCED_FIELDS, CHOICE_HELP, SB_ARMS, defaultKind, emptySbForm, emptyT1Form, errorSummary, labCommand, presetsFor,
  runSpecJson, validateSb, validateT1, visibleErrors,
} from '../../lib/labForms.js';

// Run form (increment I2). Spec section 9: fieldset groups with a title-tier legend; validation by TOUCHED state (no error on a
// pristine form, inline error after blur or after a failed submit, a summary with links after a failed submit); a sticky action
// bar whose disabled state says why; advanced options one level deeper (S16 Simplicity); long catalogues are collapsible
// groups with counts and select all / clear so 19 arms and 12 scenarios never become a 5,000 px form.
// Skill: S16 (validate inline, grouping and mapping, common path first), S1 (instant feedback), S14 (reduced motion via the shared CSS).

function FieldError({ id, message }) {
  return (
    <p id={id} className="field__error" data-field-error>
      <Icon name="error" />
      <span>{message}</span>
    </p>
  );
}

function TextField({ fid, label, value, onChange, onBlur, error, hint, inputMode, placeholder, maxLength }) {
  const eid = `${fid}-e`;
  const hid = `${fid}-h`;
  return (
    <div className="field" data-touched={error ? '' : undefined}>
      <label className="field__label" htmlFor={fid}>{label}</label>
      <input
        id={fid} type="text" inputMode={inputMode} autoComplete="off" spellCheck={false} maxLength={maxLength}
        value={value} placeholder={placeholder} onChange={(e) => onChange(e.target.value)} onBlur={onBlur}
        aria-invalid={error ? 'true' : undefined} aria-describedby={error ? `${hid} ${eid}` : hid}
      />
      {hint ? <p id={hid} className="field__hint">{hint}</p> : <span id={hid} className="visually-hidden">{label}</span>}
      {error ? <FieldError id={eid} message={error} /> : null}
    </div>
  );
}

/**
 * A list of checkboxes with a count and select all / clear. `collapsible` groups (long catalogues) are disclosures whose
 * summary carries the count, so the form stays short; they open by themselves when they hold an error that is showing.
 */
function ChoiceGroup({ fid, legend, options, selected, onChange, error, help, collapsible = false, open, onToggle, noun }) {
  const lid = `${fid}-l`;
  const eid = `${fid}-e`;
  const count = `${selected.length} of ${options.length} selected`;
  const list = (
    <div className="choice-list" role="group" aria-labelledby={lid} aria-describedby={error ? eid : undefined}>
      <div className="choice-list__tools">
        <button type="button" className="btn btn--ghost" onClick={() => onChange([...options])} aria-label={`Select all ${options.length} ${noun || legend}`}>Select all</button>
        <button type="button" className="btn btn--ghost" onClick={() => onChange([])} aria-label={`Clear the selection of ${noun || legend}`}>Clear</button>
        <span className="lab-choice-count" aria-live="polite">{count}</span>
      </div>
      <div className="lab-choices">
        {options.map((o) => (
          <label key={o} className="choice">
            <input
              type="checkbox" checked={selected.includes(o)}
              onChange={(e) => onChange(e.target.checked ? [...selected, o] : selected.filter((x) => x !== o))}
            />
            <span className={collapsible ? 'mono' : undefined}>{o}{help && help[o] ? <span className="lab-hint lab-choice__help"> {help[o]}</span> : null}</span>
          </label>
        ))}
      </div>
      {collapsible ? <p className="lab-hint" data-ids-note>Ids exactly as the gateway lists them; it provides no names or descriptions for them.</p> : null}
    </div>
  );
  return (
    <div className="field lab-choice-group" id={fid} data-touched={error ? '' : undefined} data-choice-group={fid.split('-').pop()}>
      {collapsible ? (
        <details className="disclosure" open={open} onToggle={(e) => onToggle(e.currentTarget.open)}>
          <summary>
            <span id={lid} className="lab-choice-group__title">{legend}</span>
            <span className="lab-choice-group__count">{selected.length ? count : `none selected, ${options.length} available`}</span>
          </summary>
          {list}
        </details>
      ) : (
        <>
          <span id={lid} className="field__label">{legend}</span>
          {list}
        </>
      )}
      {error ? <FieldError id={eid} message={error} /> : null}
    </div>
  );
}

function RadioList({ fid, legend, name, options, value, onChange, error, help }) {
  const eid = `${fid}-e`;
  return (
    <fieldset className="lab-plain-fieldset field" id={fid} aria-describedby={error ? eid : undefined} data-touched={error ? '' : undefined}>
      <legend className="field__label">{legend}</legend>
      <div className="choice-list">
        {options.map((o) => (
          <label key={o.value} className="choice">
            <input type="radio" name={`${fid}-${name}`} value={o.value} checked={value === o.value} onChange={() => onChange(o.value)} />
            <span>{o.label}{help && help[o.value] ? <span className="lab-hint lab-choice__help"> {help[o.value]}</span> : null}</span>
          </label>
        ))}
      </div>
      {error ? <FieldError id={eid} message={error} /> : null}
    </fieldset>
  );
}

function rangeHint(o, extra) {
  if (!o || !Number.isFinite(o.min) || !Number.isFinite(o.max)) return extra || '';
  return `${o.min} to ${o.max}. ${extra || ''}`.trim();
}

function Presets({ presets, onApply }) {
  return (
    <div className="lab-presets" role="group" aria-label="Presets: fill the form (nothing is started)">
      {presets.map((p) => (
        <button key={p.id} type="button" className="btn lab-preset" onClick={() => onApply(p)} data-preset={p.id}>
          <span className="lab-preset__name">{p.label}</span>
          <span className="lab-preset__text">{p.summary}</span>
        </button>
      ))}
    </div>
  );
}

function AdvancedGroup({ open, onToggle, count, children }) {
  return (
    <details className="disclosure lab-advanced" open={open} onToggle={(e) => onToggle(e.currentTarget.open)} data-advanced>
      <summary><span className="lab-choice-group__title">Advanced settings</span><span className="lab-choice-group__count">{count}</span></summary>
      <div className="lab-advanced__body">{children}</div>
    </details>
  );
}

const KIND_INFO = Object.freeze({
  scenario_b: { short: 'Scenario B', next: 'Starts real gateway processes on this machine; a run takes minutes.' },
  t1_matrix: { short: 'simulation matrix', next: 'Runs a deterministic simulation; a run usually takes seconds.' },
});

/**
 * Run form body. `caps` is capabilitiesView(). `onSubmit(kind, spec)` returns nothing; `submitting` and `submitError`
 * are controlled by the container. The Start button stays enabled on an incomplete form (pressing it lists what is missing);
 * it is aria-disabled, with the reason beside it, only when starting is impossible (kind unavailable, a run is active, sending).
 */
export function RunFormBody({ caps, roles = null, onSubmit, submitting = false, submitError = null, initialKind }) {
  const uid = useId();
  const [kind, setKind] = useState(() => initialKind || defaultKind(caps));
  const [t1, setT1] = useState(() => emptyT1Form());
  const [sb, setSb] = useState(() => emptySbForm(caps));
  const [touched, setTouched] = useState({});
  const [attempted, setAttempted] = useState(false);
  const [open, setOpen] = useState({});
  const [advOpen, setAdvOpen] = useState(false);
  const [pending, setPending] = useState(null);
  const [summaryTick, setSummaryTick] = useState(0);
  const summaryRef = useRef(null);
  const result = useMemo(() => (kind === 't1_matrix' ? validateT1(t1, caps) : validateSb(sb, caps)), [kind, t1, sb, caps]);
  const errors = result.errors;
  const shown = visibleErrors(errors, touched, attempted);
  const summary = errorSummary(kind, errors);
  const spec = result.spec;
  const json = spec ? runSpecJson(kind, spec) : null;
  const command = spec ? labCommand(kind, spec) : null;
  const available = kind === 't1_matrix' ? caps.t1.available : caps.sb.available;
  const submitId = useId();
  const roleCaps = roleCapabilities(roles);
  const cannotOperate = roleCaps !== null && !roleCaps.canOperate;
  const operatorOnly = roleCaps !== null && roleCaps.canOperate && !roleCaps.canRead;
  const activeBlock = caps.activeRun ? `A run is already active (${caps.activeRun.runId}); the gateway accepts one at a time.` : null;
  const disabledReason = submitting ? 'Waiting for the gateway to accept the run.' : !available ? 'This kind of run is not available here.' : activeBlock;
  const fid = (name) => `${uid}-${name}`;
  const touch = (name) => setTouched((t) => (t[name] ? t : { ...t, [name]: true }));
  const setT1p = (p) => setT1((f) => ({ ...f, ...p }));
  const setSbp = (p) => setSb((f) => ({ ...f, ...p }));
  const form = kind === 't1_matrix' ? t1 : sb;
  const setForm = kind === 't1_matrix' ? setT1p : setSbp;
  const field = (name) => ({
    fid: fid(name), value: form[name], error: shown[name],
    onChange: (v) => setForm({ [name]: v }), onBlur: () => touch(name),
  });
  const group = (name) => ({ fid: fid(name), error: shown[name], selected: form[name], onChange: (v) => { setForm({ [name]: v }); touch(name); } });
  const applyPreset = (p) => {
    setForm(p.patch);
    setTouched({});
    setAttempted(false);
  };

  useEffect(() => {
    if (summaryTick > 0 && summaryRef.current) summaryRef.current.focus();
  }, [summaryTick]);
  useEffect(() => {
    if (!pending) return;
    const el = document.getElementById(pending);
    if (el) {
      const target = el.matches('input,button,select,textarea,summary') ? el : el.querySelector('input,select,textarea,summary');
      if (target) target.focus();
    }
    setPending(null);
  }, [pending]);

  const submit = (e) => {
    e.preventDefault();
    if (submitting || disabledReason) return;
    if (!result.valid) {
      setAttempted(true);
      if (ADVANCED_FIELDS[kind].some((k) => errors[k])) setAdvOpen(true);
      setOpen((o) => ({ ...o, ...Object.fromEntries(Object.keys(errors).map((k) => [k, true])) }));
      setSummaryTick((n) => n + 1);
      return;
    }
    onSubmit(kind, spec);
  };

  const goField = (e, name) => {
    e.preventDefault(); // the hash belongs to the router
    setOpen((o) => ({ ...o, [name]: true }));
    if (ADVANCED_FIELDS[kind].includes(name)) setAdvOpen(true);
    setPending(fid(name));
  };

  const l = (caps.sb && caps.sb.limits) || {};
  const t1c = caps.t1;
  const setInfo = t1c.seedSets.find((s) => s.name === t1.seedSet);
  const n = summary.length;
  const things = n === 1 ? '1 thing' : `${n} things`;
  // The sticky bar carries ONE short status; the longer explanation (what happens next, what is still needed) sits in Review.
  const helpText = submitting ? 'Waiting for the gateway to accept the run.'
    : disabledReason || (attempted && !result.valid ? `${things} to fix.`
      : !result.valid ? `${things} still needed.` : 'Ready to start.');
  const nextText = `${KIND_INFO[kind].next} You can cancel it from the run page. Starting needs the Operator role when the gateway uses tokens.`;
  const stillNeeded = !result.valid && !attempted ? `Still needed: ${summary.map((s) => s.label).join(', ')}.` : null;
  // Duration estimate (DERIVED): only when every phase length is in the form. The gateway's defaults are not known to this page, so
  // nothing is guessed. It is a lower bound: start-up, drain and shut-down add time. Scenario B only (a matrix cell varies in cost).
  const estimate = kind === 'scenario_b' && spec
    ? plannedMinimum({ warmup_s: spec.warmup_s, healthy_s: spec.healthy_s, degraded_s: spec.degraded_s, recovery_s: spec.recovery_s }, spec.repetitions * spec.arms.length)
    : null;
  const debugBuild = kind === 'scenario_b' && caps.sb && (caps.sb.buildProfile === 'debug' || caps.sb.gatewayProfile === 'debug');

  return (
    <form className="form lab-form" noValidate aria-label="Run specification" onSubmit={submit}>
      {attempted && summary.length ? (
        <div className="form-summary" role="alert" tabIndex={-1} ref={summaryRef} data-error-summary>
          <strong>{summary.length === 1 ? '1 thing needs' : `${summary.length} things need`} attention before this run can start</strong>
          <ul>
            {summary.map((s) => (
              <li key={s.field}><a href={`#${fid(s.field)}`} onClick={(e) => goField(e, s.field)}>{s.label}</a>: {s.message}</li>
            ))}
          </ul>
        </div>
      ) : null}

      <fieldset className="field-group">
        <legend>What to run</legend>
        <div className="choice-list lab-kinds">
          <label className="choice lab-kind" data-kind-option="scenario_b">
            <input type="radio" name={`${uid}-kind`} checked={kind === 'scenario_b'} onChange={() => setKind('scenario_b')} />
            <span className="lab-kind__body">
              <span className="lab-kind__title">Scenario B: origin slowdown <TierBadge tier="MEASURED_LOOPBACK" /></span>
              <span className="lab-hint">Real gateway processes on this machine against a scripted origin that slows down. Takes minutes. One host, loopback only.</span>
            </span>
          </label>
          <label className="choice lab-kind" data-kind-option="t1_matrix">
            <input type="radio" name={`${uid}-kind`} checked={kind === 't1_matrix'} onChange={() => setKind('t1_matrix')} />
            <span className="lab-kind__body">
              <span className="lab-kind__title">Simulation matrix <TierBadge tier="SIMULATED" /></span>
              <span className="lab-hint">Deterministic virtual-time simulation on the real cache code. Fast. Not a measurement of any network or host.</span>
            </span>
          </label>
        </div>
      </fieldset>

      {!available ? (
        <StateBlock kind="unavailable" title="This kind of run is not available here">
          {(kind === 't1_matrix' ? caps.t1.reason : caps.sb.reason) || 'The gateway did not say why.'}
        </StateBlock>
      ) : kind === 'scenario_b' ? (
        <>
          <fieldset className="field-group">
            <legend>Settings</legend>
            <p className="field-group__help">Start from a preset or set the values yourself. A preset only fills the form.</p>
            <Presets presets={presetsFor('scenario_b', caps)} onApply={applyPreset} />
            <TextField {...field('repetitions')} label="Repetitions per arm" inputMode="numeric" hint={`${rangeHint(l.repetitions)} ${CHOICE_HELP.repetitions}`} />
            <ChoiceGroup {...group('arms')} legend="Arms" options={[...SB_ARMS]} help={CHOICE_HELP.arms} noun="arms" />
            <AdvancedGroup open={advOpen} onToggle={setAdvOpen} count="phase lengths, arrival rate, key catalogue, caps (all optional)">
              <div className="lab-subgroup" role="group" aria-labelledby={fid('phases-h')}>
                <h4 id={fid('phases-h')}>Phase lengths (seconds, optional)</h4>
                <p className="field__hint">{CHOICE_HELP.phases} Allowed: {rangeHint(l.phase_seconds)}</p>
                <div className="lab-fieldgrid">
                  <TextField {...field('warmup')} label="Warm-up" inputMode="numeric" hint="Before measurement." />
                  <TextField {...field('healthy')} label="Healthy" inputMode="numeric" hint="Origin at normal speed." />
                  <TextField {...field('degraded')} label="Degraded" inputMode="numeric" hint="Origin slowed down." />
                  <TextField {...field('recovery')} label="Recovery" inputMode="numeric" hint="Origin back to normal." />
                </div>
              </div>
              <div className="lab-subgroup" role="group" aria-labelledby={fid('load-h')}>
                <h4 id={fid('load-h')}>Workload</h4>
                <div className="lab-fieldgrid">
                  <TextField {...field('rate')} label="Arrival rate (requests per second)" inputMode="decimal" hint={`${rangeHint(l.rate_rps)} ${CHOICE_HELP.rate}`} />
                  <TextField {...field('keys')} label="Key catalogue size" inputMode="numeric" hint={`${rangeHint(l.n_keys)} ${CHOICE_HELP.keys}`} />
                </div>
              </div>
              <div className="lab-subgroup" role="group" aria-labelledby={fid('caps-h')}>
                <h4 id={fid('caps-h')}>Cap settings (optional)</h4>
                <p className="field__hint">{CHOICE_HELP.caps} Allowed: {rangeHint(l.caps)}</p>
                <div className="lab-fieldgrid">
                  <TextField {...field('staticCap')} label="Static cap" inputMode="numeric" hint="STATIC-CAP arm." />
                  <TextField {...field('aimdMin')} label="AIMD minimum" inputMode="numeric" hint="AIMD-CAP lower bound." />
                  <TextField {...field('aimdMax')} label="AIMD maximum" inputMode="numeric" hint="AIMD-CAP upper bound." />
                </div>
              </div>
            </AdvancedGroup>
          </fieldset>
        </>
      ) : (
        <>
          <fieldset className="field-group">
            <legend>Experiment design</legend>
            <p className="field-group__help">Choose what the simulation compares. The groups are collapsed; each shows how many you picked.</p>
            <Presets presets={presetsFor('t1_matrix', caps)} onApply={applyPreset} />
            <ChoiceGroup {...group('scenarios')} legend="Scenarios" noun="scenarios" options={t1c.scenarios} collapsible open={Boolean(open.scenarios)} onToggle={(v) => setOpen((o) => ({ ...o, scenarios: v }))} />
            <ChoiceGroup {...group('families')} legend="Workload families" noun="workload families" options={t1c.families} collapsible open={Boolean(open.families)} onToggle={(v) => setOpen((o) => ({ ...o, families: v }))} />
            <ChoiceGroup {...group('arms')} legend="Arms (policies compared)" noun="arms" options={t1c.arms} collapsible open={Boolean(open.arms)} onToggle={(v) => setOpen((o) => ({ ...o, arms: v }))} />
          </fieldset>
          <fieldset className="field-group">
            <legend>Settings</legend>
            <RadioList
              {...{ fid: fid('seedSet'), error: shown.seedSet }} legend="Seed set" name="seedset" value={t1.seedSet} onChange={(v) => { setT1p({ seedSet: v }); touch('seedSet'); }}
              options={['T', 'D'].filter((n) => t1c.seedSets.some((s) => s.name === n)).map((n) => ({ value: n, label: `${n}${t1c.seedSets.find((s) => s.name === n)?.size ? ` (${t1c.seedSets.find((s) => s.name === n).size} seeds)` : ''}` }))}
              help={CHOICE_HELP.seedSet}
            />
            <p className="field__hint">{t1c.seedSetsNote || 'Only the tuning and development sets are offered. The sealed set is not available through the Lab.'}</p>
            <TextField
              {...field('nSeeds')} label="Number of seeds" inputMode="numeric"
              hint={`The first N seeds of the chosen set. ${rangeHint({ min: 1, max: Math.min(t1c.limits?.max_seeds ?? Infinity, setInfo?.size ?? Infinity) })}`}
            />
            <RadioList
              {...{ fid: fid('profile'), error: shown.profile }} legend="Profile" name="profile" value={t1.profile} onChange={(v) => { setT1p({ profile: v }); touch('profile'); }}
              options={t1c.profiles.map((p) => ({ value: p, label: p }))} help={CHOICE_HELP.profile}
            />
            <AdvancedGroup open={advOpen} onToggle={setAdvOpen} count="arm parameters, Zipf alpha, run name, raw files (all optional)">
              <RadioList
                {...{ fid: fid('armParams'), error: shown.armParams }} legend="Arm parameters" name="armparams" value={t1.armParams} onChange={(v) => { setT1p({ armParams: v }); touch('armParams'); }}
                options={['untuned', 'frozen'].map((p) => ({ value: p, label: p }))} help={CHOICE_HELP.armParams}
              />
              <TextField {...field('wzAlpha')} label="Zipf alpha for W-Z (optional)" inputMode="decimal" hint="Leave empty for the family default. Applies to W-Z only; must be greater than 0 and at most 10." />
              <TextField {...field('name')} label="Run name (optional)" maxLength={64} hint="Letters, digits, space, dot, underscore, hyphen; up to 64 characters. Recorded in the manifest." />
              <label className="choice">
                <input type="checkbox" checked={t1.recordRaw} onChange={(e) => setT1p({ recordRaw: e.target.checked })} />
                <span>Record raw per-request files<span className="lab-hint lab-choice__help"> {CHOICE_HELP.recordRaw}</span></span>
              </label>
            </AdvancedGroup>
          </fieldset>
        </>
      )}

      <fieldset className="field-group">
        <legend>Review</legend>
        <p className="lab-next" data-next-help>{nextText}{stillNeeded ? ` ${stillNeeded}` : ''}</p>
        {result.cells !== null && result.cells !== undefined ? (
          <p className="field__hint lab-estimate" data-cell-estimate>This selection is {result.cells} cell(s) (seeds x scenarios x families x arms). <ProvenanceBadge src="derived" qualifier="from your selection" /> The gateway gives no basis for a time estimate: a cell varies in cost.</p>
        ) : null}
        {kind === 'scenario_b' && available ? (
          estimate && estimate.state === 'value' ? (
            <p className="lab-estimate" data-duration-estimate="value">
              Planned minimum: at least <strong className="num">{estimate.text}</strong>
              <ProvenanceBadge src="derived" qualifier="from the phase lengths you entered" />
              <span className="lab-hint">{spec.repetitions * spec.arms.length} run(s) x {estimate.perRunSeconds} s of phases. Start-up, drain and shut-down add time this page cannot measure.</span>
            </p>
          ) : (
            <p className="lab-estimate lab-hint" data-duration-estimate="unavailable">
              Duration: not derived. {spec ? 'Fill in all four phase lengths (Advanced settings) to see the planned minimum; the gateway defaults are not known to this page.' : 'The form is not complete yet.'}
            </p>
          )
        ) : null}
        {result.warnings && result.warnings.length ? (
          <Notice tone="warn" title="Before you start" role="status">
            <ul className="lab-reasons">{result.warnings.map((w) => <li key={w}>{w}</li>)}</ul>
          </Notice>
        ) : null}
        {cannotOperate ? (
          <Notice tone="warn" title="This token cannot start runs" role="status">
            The roles in your token ({roles.length ? roles.join(', ') : 'none'}) do not include Operator (scenario:operate) or admin. The gateway will refuse to start a run with a 403. Shown from the token as pasted; the gateway decides.
          </Notice>
        ) : null}
        {operatorOnly ? (
          <Notice tone="info" title="This token can start runs but cannot read them" role="status" id={`${submitId}-ro`}>
            Your token ({roles.join(', ')}) has the Operator role but not Observer (telemetry:read). The gateway can accept a run, but this token cannot then open the run page, its progress or its results (a 403 is expected there). Use an admin token, or note the run id and open it with an Observer token.
          </Notice>
        ) : null}
        {activeBlock ? <Notice tone="warn" title="A run is active" role="status">{activeBlock}</Notice> : null}
        {submitError ? <Notice tone="critical" title={submitError.title} role="alert">{submitError.fix}{submitError.serverMessage ? <> Gateway message: <span className="mono">{submitError.serverMessage}</span></> : null}</Notice> : null}
        <details className="disclosure" data-request-disclosure>
          <summary><span className="lab-choice-group__title">Exact request and terminal command</span><span className="lab-choice-group__count">for checking or repeating the run</span></summary>
          <div className="lab-request">
            <p className="field__hint">What this page will send to POST /api/v1/lab/runs. The gateway validates it again.</p>
            {json ? (
              <>
                <pre className="lab-verbatim" tabIndex={0} aria-label="Run specification JSON" data-runspec>{json}</pre>
                <div className="lab-copy"><CopyButton text={json} label="Copy JSON" /></div>
              </>
            ) : <p className="field__hint" data-runspec-empty>The request is shown once the form is complete.</p>}
            <p className="field__hint">The same run from a terminal uses the same experiment code. Choose the output folder yourself.</p>
            {command ? (
              <>
                <pre className="lab-verbatim" tabIndex={0} aria-label="Equivalent phantom-lab command" data-command>{command}</pre>
                <div className="lab-copy"><CopyButton text={command} label="Copy command" /></div>
                {kind === 'scenario_b' ? <p className="field__hint">From a terminal Scenario B also takes --gateway PATH, the gateway binary to start. Use a release build for a run that can be labelled VALID.</p> : null}
              </>
            ) : <p className="field__hint" data-command-empty>The command is shown once the form is complete.</p>}
          </div>
        </details>
      </fieldset>

      <div className="form-bar" data-form-bar>
        <button type="submit" className="btn btn--primary" aria-disabled={disabledReason ? 'true' : undefined} aria-busy={submitting ? 'true' : undefined} aria-describedby={submitId}>
          {submitting ? 'Starting' : 'Start run'}
        </button>
        <span id={submitId} className="form-bar__next" data-submit-help data-submit-disabled={disabledReason ? 'true' : undefined}>
          {disabledReason ? <DisabledReason>{helpText}</DisabledReason> : helpText}
          {debugBuild ? <span className="form-bar__debug" data-debug-warning><Icon name="warn" />Debug build: this run will be labelled EXPLORATORY and checks mechanics only.</span> : null}
        </span>
      </div>
    </form>
  );
}
