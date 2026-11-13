// Run forms: validation that mirrors the Lab API limits (docs/user/lab_api.md "Limits"), the exact RunSpec JSON,
// and the equivalent `phantom-lab` command. The gateway stays authoritative: it validates again and its answer wins.
// Limits are read from GET /capabilities (so they follow the running gateway); nothing is hard-coded except the
// charset rules the API documents (run name, W-Z alpha range) and the fact that seed set S is never offered.

export const NAME_PATTERN = /^[A-Za-z0-9._ -]{1,64}$/;
export const ALPHA_MAX = 10; // documented: W-Z(alpha) needs a finite alpha in (0, 10]
export const OFFERED_SEED_SETS = Object.freeze(['T', 'D']); // the sealed set is refused by the API and never offered here
export const SB_ARMS = Object.freeze(['BASE', 'STATIC-CAP', 'AIMD-CAP']);

const INT = /^[0-9]+$/;
const DEC = /^[0-9]+(\.[0-9]+)?$/;

export function emptyT1Form() {
  return { name: '', scenarios: [], families: [], arms: [], wzAlpha: '', seedSet: 'D', nSeeds: '', profile: 'smoke', armParams: 'untuned', recordRaw: false };
}

export function emptySbForm(caps) {
  const l = caps?.sb?.limits || {};
  const dflt = (o) => (o && Number.isFinite(o.default) ? String(o.default) : '');
  return {
    repetitions: dflt(l.repetitions),
    arms: [...SB_ARMS],
    warmup: '', healthy: '', degraded: '', recovery: '',
    rate: dflt(l.rate_rps),
    keys: dflt(l.n_keys),
    staticCap: '', aimdMin: '', aimdMax: '',
  };
}

function toInt(s) {
  const t = String(s ?? '').trim();
  return INT.test(t) ? Number(t) : null;
}

function range(errors, field, label, raw, { min, max, required = false, integer = true }) {
  const t = String(raw ?? '').trim();
  if (t === '') {
    if (required) errors[field] = `${label} is required.`;
    return undefined;
  }
  const ok = integer ? INT.test(t) : DEC.test(t);
  if (!ok) {
    errors[field] = integer ? `${label} must be a whole number.` : `${label} must be a number.`;
    return undefined;
  }
  const n = Number(t);
  if (min !== undefined && max !== undefined && (n < min || n > max)) {
    errors[field] = `${label} must be between ${min} and ${max}.`;
    return undefined;
  }
  if (min !== undefined && n < min) {
    errors[field] = `${label} must be at least ${min}.`;
    return undefined;
  }
  return n;
}

/** Validate the simulation-matrix form. Returns { valid, errors, spec, cells }. */
export function validateT1(form, caps) {
  const errors = {};
  const t1 = caps?.t1;
  if (!t1 || !t1.available) {
    return { valid: false, errors: { _form: 'The gateway does not offer simulation runs right now.' }, spec: null, cells: null };
  }
  const lim = t1.limits || {};
  const name = String(form.name ?? '').trim();
  if (name && !NAME_PATTERN.test(name)) errors.name = 'Use 1 to 64 characters from letters, digits, space, dot, underscore, hyphen.';
  const pick = (field, label, chosen, catalogue) => {
    if (!chosen.length) errors[field] = `Choose at least one ${label}.`;
    else if (chosen.some((c) => !catalogue.includes(c))) errors[field] = `Unknown ${label} in the selection.`;
  };
  pick('scenarios', 'scenario', form.scenarios, t1.scenarios);
  pick('arms', 'arm', form.arms, t1.arms);
  pick('families', 'family', form.families, t1.families);

  const sealed = /^\s*(s|sealed)\s*$/i.test(String(form.seedSet ?? ''));
  if (sealed || !OFFERED_SEED_SETS.includes(form.seedSet)) {
    errors.seedSet = sealed ? 'The sealed seed set is not available through the Lab.' : 'Choose seed set T or D.';
  }
  const setInfo = t1.seedSets.find((s) => s.name === form.seedSet);
  const maxSeeds = Math.min(Number.isFinite(lim.max_seeds) ? lim.max_seeds : Infinity, setInfo && Number.isFinite(setInfo.size) ? setInfo.size : Infinity);
  const nSeeds = range(errors, 'nSeeds', 'Number of seeds', form.nSeeds, { min: 1, max: Number.isFinite(maxSeeds) ? maxSeeds : undefined, required: true });

  let families = [...form.families];
  const alpha = String(form.wzAlpha ?? '').trim();
  if (alpha !== '') {
    if (!families.includes('W-Z')) errors.wzAlpha = 'An alpha applies to family W-Z only; select W-Z or clear this field.';
    else if (!DEC.test(alpha) || !(Number(alpha) > 0) || Number(alpha) > ALPHA_MAX) errors.wzAlpha = `Alpha must be greater than 0 and at most ${ALPHA_MAX}.`;
    else families = families.map((f) => (f === 'W-Z' ? `W-Z(${alpha})` : f));
  }
  if (!t1.profiles.includes(form.profile)) errors.profile = 'Choose one of the profiles the gateway offers.';
  if (form.armParams !== 'untuned' && form.armParams !== 'frozen') errors.armParams = 'Choose untuned or frozen.';
  else if (t1.armParams && t1.armParams[form.armParams] !== true) errors.armParams = 'The gateway reports this arm-parameter set as unavailable.';

  const cells = nSeeds !== undefined && !errors.scenarios && !errors.families && !errors.arms ? nSeeds * form.scenarios.length * families.length * form.arms.length : null;
  if (cells !== null && Number.isFinite(lim.max_cells) && cells > lim.max_cells) errors.nSeeds = `This selection would run ${cells} cells; the gateway allows at most ${lim.max_cells}. Reduce seeds or selections.`;

  const valid = Object.keys(errors).length === 0;
  const spec = valid ? {
    ...(name ? { name } : {}),
    scenarios: [...form.scenarios], families, arms: [...form.arms],
    seed_set: form.seedSet, n_seeds: nSeeds, profile: form.profile, arm_params: form.armParams, record_raw: Boolean(form.recordRaw),
  } : null;
  return { valid, errors, spec, cells, warnings: [] };
}

/** Validate the Scenario B form. Returns { valid, errors, warnings, spec }. */
export function validateSb(form, caps) {
  const errors = {};
  const warnings = [];
  const sb = caps?.sb;
  if (!sb || !sb.available) {
    return { valid: false, errors: { _form: sb && sb.reason ? `Scenario B cannot run here: ${sb.reason}` : 'The gateway does not offer Scenario B runs right now.' }, warnings, spec: null };
  }
  const l = sb.limits || {};
  const reps = range(errors, 'repetitions', 'Repetitions', form.repetitions, { min: l.repetitions?.min, max: l.repetitions?.max, required: true });
  const arms = [...form.arms];
  if (!arms.length) errors.arms = 'Choose at least one arm.';
  else if (arms.some((a) => !SB_ARMS.includes(a)) || new Set(arms).size !== arms.length) errors.arms = 'Arms must be distinct values of BASE, STATIC-CAP, AIMD-CAP.';
  const ph = l.phase_seconds || {};
  const warmup = range(errors, 'warmup', 'Warm-up seconds', form.warmup, { min: ph.min, max: ph.max });
  const healthy = range(errors, 'healthy', 'Healthy seconds', form.healthy, { min: ph.min, max: ph.max });
  const degraded = range(errors, 'degraded', 'Degraded seconds', form.degraded, { min: ph.min, max: ph.max });
  const recovery = range(errors, 'recovery', 'Recovery seconds', form.recovery, { min: ph.min, max: ph.max });
  const rate = range(errors, 'rate', 'Arrival rate', form.rate, { min: l.rate_rps?.min, max: l.rate_rps?.max, integer: false });
  const keys = range(errors, 'keys', 'Key catalogue size', form.keys, { min: l.n_keys?.min, max: l.n_keys?.max });
  const cap = l.caps || {};
  const staticCap = range(errors, 'staticCap', 'Static cap', form.staticCap, { min: cap.min, max: cap.max });
  const aimdMin = range(errors, 'aimdMin', 'AIMD minimum', form.aimdMin, { min: cap.min, max: cap.max });
  const aimdMax = range(errors, 'aimdMax', 'AIMD maximum', form.aimdMax, { min: cap.min, max: cap.max });
  if (aimdMin !== undefined && aimdMax !== undefined && aimdMin > aimdMax) errors.aimdMin = 'AIMD minimum must not exceed the maximum.';

  const needed = l.repetitions?.valid_label_needs_at_least;
  if (reps !== undefined && Number.isFinite(needed) && reps < needed) warnings.push(`Fewer than ${needed} repetitions: the result will be labelled EXPLORATORY and verdicts will be INCONCLUSIVE.`);
  if (arms.length && !arms.includes('BASE')) warnings.push('Without the BASE arm there is nothing to pair against: no verdict can be computed.');
  if (sb.buildProfile === 'debug' || sb.gatewayProfile === 'debug') warnings.push('This gateway is a debug build: runs are mechanics checks and are labelled EXPLORATORY regardless of the settings.');

  const valid = Object.keys(errors).length === 0;
  const spec = valid ? {
    repetitions: reps,
    arms,
    ...(warmup !== undefined ? { warmup_s: warmup } : {}),
    ...(healthy !== undefined ? { healthy_s: healthy } : {}),
    ...(degraded !== undefined ? { degraded_s: degraded } : {}),
    ...(recovery !== undefined ? { recovery_s: recovery } : {}),
    ...(rate !== undefined ? { rate_rps: rate } : {}),
    ...(keys !== undefined ? { n_keys: keys } : {}),
    ...(staticCap !== undefined ? { static_cap: staticCap } : {}),
    ...(aimdMin !== undefined ? { aimd_min: aimdMin } : {}),
    ...(aimdMax !== undefined ? { aimd_max: aimdMax } : {}),
  } : null;
  return { valid, errors, warnings, spec };
}

/** The exact body POSTed to /api/v1/lab/runs, pretty-printed. */
export function runSpecJson(kind, spec) {
  return JSON.stringify({ kind, spec }, null, 2);
}

function shellQuote(s) {
  const t = String(s);
  return /^[A-Za-z0-9._,:=/\\-]+$/.test(t) ? t : `"${t.replace(/(["\\$`])/g, '\\$1')}"`;
}

/**
 * Equivalent `phantom-lab` invocation (same experiment code, run from a terminal instead of through the gateway).
 * The out directory is a placeholder the person must choose. Scenario B in the terminal also accepts --gateway
 * (the gateway binary to spawn); use a release build for a VALID label.
 */
export function labCommand(kind, spec) {
  if (!spec) return '';
  const parts = ['phantom-lab'];
  if (kind === 't1_matrix') {
    parts.push('run', '--scenarios', shellQuote(spec.scenarios.join(',')), '--families', shellQuote(spec.families.join(',')), '--arms', shellQuote(spec.arms.join(',')),
      '--seed-set', spec.seed_set, '--n-seeds', String(spec.n_seeds), '--profile', spec.profile, '--arm-params', spec.arm_params);
    if (spec.name) parts.push('--name', shellQuote(spec.name));
    if (spec.record_raw) parts.push('--raw');
    parts.push('--out', 'lab-results/my-run');
    return parts.join(' ');
  }
  parts.push('scenario-b', '--reps', String(spec.repetitions), '--arms', shellQuote(spec.arms.join(',')));
  const ms = (v) => String(v * 1000);
  if (spec.warmup_s !== undefined) parts.push('--warmup-ms', ms(spec.warmup_s));
  if (spec.healthy_s !== undefined) parts.push('--healthy-ms', ms(spec.healthy_s));
  if (spec.degraded_s !== undefined) parts.push('--degraded-ms', ms(spec.degraded_s));
  if (spec.recovery_s !== undefined) parts.push('--recovery-ms', ms(spec.recovery_s));
  if (spec.rate_rps !== undefined) parts.push('--rate', String(spec.rate_rps));
  if (spec.n_keys !== undefined) parts.push('--keys', String(spec.n_keys));
  if (spec.static_cap !== undefined) parts.push('--static-cap', String(spec.static_cap));
  if (spec.aimd_min !== undefined) parts.push('--aimd-min', String(spec.aimd_min));
  if (spec.aimd_max !== undefined) parts.push('--aimd-max', String(spec.aimd_max));
  parts.push('--out', 'lab-results/scenario_b/my-run');
  return parts.join(' ');
}

/** Explanations shown next to each choice. Text only; no numbers. */
export const CHOICE_HELP = Object.freeze({
  seedSet: {
    T: 'Tuning seeds: the set used to choose arm parameters. Results on T are exploratory.',
    D: 'Development seeds: the set for trying things out. Results on D are exploratory and are not the registered evaluation.',
  },
  profile: {
    smoke: 'Short simulated horizon, for checking that a selection runs.',
    standard: 'The registered horizon. Slower.',
  },
  armParams: {
    untuned: 'Library defaults for every arm.',
    frozen: 'The frozen arm parameters retained from the tuning round (a retained file on the gateway host).',
  },
  recordRaw: 'Also write per-request and per-fetch CSV files. Much larger artifacts.',
  repetitions: 'Independent runs per arm. Every repetition starts a fresh gateway process. A label of VALID needs a minimum number of them (shown in the hint).',
  arms: {
    BASE: 'No origin-concurrency cap. The reference every other arm is paired against.',
    'STATIC-CAP': 'A fixed limit on concurrent origin fetches. Requests over the limit are shed with 503.',
    'AIMD-CAP': 'A limit that moves between a minimum and a maximum. It may settle near its minimum and then behave like a static cap.',
  },
  phases: 'Phase lengths in seconds. Leave a field empty to use the gateway default; the run page shows the values that were used.',
  rate: 'Open-loop Poisson arrival rate in requests per second.',
  keys: 'Number of distinct keys in the Zipf-distributed catalogue.',
  caps: 'Leave empty to use the gateway default. The run page shows the values that were used.',
});

// ---------------------------------------------------------------------------------------------------------------
// Form behaviour added for the Lab hierarchy increment (I2). All pure, so it is unit tested without a browser.

/** The kind the form opens on: Scenario B when the gateway offers it, else the simulation matrix. */
export function defaultKind(caps) {
  if (caps && caps.sb && caps.sb.available) return 'scenario_b';
  return 't1_matrix';
}

/**
 * Validation is by touched state (spec section 9). A field's error is shown only when the field was touched (blurred or
 * changed) or after a submit attempt. A pristine form shows no error at all.
 */
export function visibleErrors(errors, touched, attempted) {
  const out = {};
  for (const [k, v] of Object.entries(errors || {})) {
    if (attempted || (touched && touched[k])) out[k] = v;
  }
  return out;
}

/** Human names of the form fields, used by the error summary and the "what is missing" text. */
export const FIELD_LABELS = Object.freeze({
  name: 'Run name', scenarios: 'Scenarios', families: 'Workload families', arms: 'Arms', wzAlpha: 'Zipf alpha for W-Z', seedSet: 'Seed set',
  nSeeds: 'Number of seeds', profile: 'Profile', armParams: 'Arm parameters',
  repetitions: 'Repetitions per arm', warmup: 'Warm-up', healthy: 'Healthy', degraded: 'Degraded', recovery: 'Recovery',
  rate: 'Arrival rate', keys: 'Key catalogue size', staticCap: 'Static cap', aimdMin: 'AIMD minimum', aimdMax: 'AIMD maximum',
});

/** Fields that live in the collapsed "Advanced" group of each form; a failed submit opens the group when one of them has an error. */
export const ADVANCED_FIELDS = Object.freeze({
  t1_matrix: ['name', 'wzAlpha', 'armParams'],
  scenario_b: ['warmup', 'healthy', 'degraded', 'recovery', 'rate', 'keys', 'staticCap', 'aimdMin', 'aimdMax'],
});

/** Error summary entries `[{ field, label, message }]` in form order (links in the summary point at the field). */
export function errorSummary(kind, errors) {
  const order = kind === 't1_matrix'
    ? ['scenarios', 'families', 'arms', 'seedSet', 'nSeeds', 'profile', 'armParams', 'wzAlpha', 'name']
    : ['repetitions', 'arms', 'warmup', 'healthy', 'degraded', 'recovery', 'rate', 'keys', 'staticCap', 'aimdMin', 'aimdMax'];
  const keys = Object.keys(errors || {});
  const sorted = [...order.filter((k) => keys.includes(k)), ...keys.filter((k) => !order.includes(k))];
  return sorted.map((field) => ({ field, label: FIELD_LABELS[field] || (field === '_form' ? 'This form' : field), message: errors[field] }));
}

/**
 * Presets fill the form; they never start a run. Each lists exactly what it sets, so nothing is hidden.
 * Values come from the gateway's own limits where they exist; nothing here is a recommendation about results.
 */
export function presetsFor(kind, caps) {
  if (kind === 'scenario_b') {
    const l = (caps && caps.sb && caps.sb.limits) || {};
    const reps = Number.isFinite(l.repetitions?.valid_label_needs_at_least) ? String(l.repetitions.valid_label_needs_at_least)
      : Number.isFinite(l.repetitions?.default) ? String(l.repetitions.default) : '';
    return [
      {
        id: 'quick', label: 'Quick check',
        summary: '1 repetition, BASE and STATIC-CAP, 1 s phases. Result is EXPLORATORY.',
        patch: { repetitions: '1', arms: ['BASE', 'STATIC-CAP'], warmup: '1', healthy: '1', degraded: '1', recovery: '1' },
      },
      {
        id: 'full', label: 'All three arms',
        summary: `${reps ? `${reps} repetitions (what a VALID label needs), ` : ''}all three arms, default phases. VALID also needs a release build.`,
        patch: { repetitions: reps, arms: [...SB_ARMS], warmup: '', healthy: '', degraded: '', recovery: '' },
      },
    ];
  }
  const t1 = (caps && caps.t1) || {};
  const first = (a) => (Array.isArray(a) && a.length ? [a[0]] : []);
  return [
    {
      id: 'smallest', label: 'Smallest run',
      summary: 'First scenario, family and arm, seed set D, 1 seed, smoke. Checks that a run works; says nothing about the comparison.',
      patch: { scenarios: first(t1.scenarios), families: first(t1.families), arms: first(t1.arms), seedSet: 'D', nSeeds: '1', profile: (t1.profiles || []).includes('smoke') ? 'smoke' : ((t1.profiles || [])[0] || 'smoke') },
    },
  ];
}
