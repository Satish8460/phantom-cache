# Design Tokens

**Status:** updated by increment I1 (UI/UX remediation, 2026-10-30) to match `docs/design/phantomcache_design_system_spec.md` (the shared specification, which wins on any conflict). Earlier text (M8-05, 2026-10-29) is replaced; the old values are listed only as deprecated aliases. **Source of truth:** `ui/src/styles/tokens.css` (CSS custom properties, no preprocessor, no dependency). `app.css` uses tokens only; tests fail on a hex colour in `app.css` or any JSX file (`forbidden.test.mjs`) and on the rules in `ui/test/tokens.test.mjs` (regression guards, not evidence of visual quality). Skill sections S1 to S17 refer to `.claude/skills/apple-design/SKILL.md`.

## 1. Themes (single source)

| Mechanism | Behaviour |
|---|---|
| Declaration | Every colour token is declared once as `light-dark(<light>, <dark>)` on `:root`. The duplicated dark block of earlier versions is gone. |
| Default | `color-scheme: light dark`: follows the OS preference. |
| Manual | `<html data-theme="light|dark">` sets `color-scheme: light` or `dark` (the header button in `src/lib/theme.js`, persisted in `localStorage` when available). |
| Browser baseline | `light-dark()`: Chrome 123, Safari 17.5, Firefox 120. **No `@supports` fallback is provided**: a fallback needs a second copy of the palette, which is exactly the duplication this change removed. In an older browser the colour tokens are invalid at computed-value time and the page falls back to the browser's default colours (legible, unstyled). Accepting or rejecting this is a human decision (UNVERIFIED in a real older browser). |
| `check-contrast.mjs` | Parses both values from each `light-dark()` declaration; there is no longer a "dark consistency" check because there is one declaration. |

## 2. Colour roles

| Role | Tokens | Use |
|---|---|---|
| Surface (elevation, section 4) | `--bg` L0, `--surface` L1, `--surface-2` wells, `--surface-raised` L2, `--surface-overlay` L3 | Dark theme separates levels by lighter steps, not heavy shadows |
| Text | `--text`, `--text-2`, `--text-3` | 4.5:1 on every surface level and on the interaction tints |
| Border | `--border` (decorative), `--border-strong` (3:1, controls) | `prefers-contrast: more` maps `--border` to `--border-strong` |
| Accent | `--accent`, `--accent-contrast`, `--focus` | Links, primary button, focus ring |
| Interaction | `--tint-hover`, `--tint-press` (translucent overlays), `--scrim` (dialog dimming, no blur) | Hover and pointer-down on buttons, rows, tabs, nav, choices |
| Status | `--ok`, `--warn`, `--critical`, `--info` and `-bg` | Always paired with a glyph and text |
| Provenance | `--prov-measured` (displayed REAL), `-derived`, `-simulated`, `-replayed`, `-predicted`, `-unavailable` and `-bg`; `--prov-inferred*` are deprecated aliases of derived | Badge colour; badges also differ by glyph and border pattern |
| Chart | `--chart-line`, `--chart-grid` | Single-series lines and bars |

Provenance hues are never the only encoding and never express severity.

## 3. Typography (S15)

System stacks only (`--font-sans`, `--font-mono`); no web fonts, no Apple fonts. Sizes in `rem`. Size, leading and tracking change together; tracking is negative for large tiers, 0 for body, slightly positive for small ones. Numerals are tabular on the whole body.

| Tier | Size | Leading | Tracking | Weight | Tokens | Use |
|---|---|---|---|---|---|---|
| display | 1.75rem (28); 1.5rem at 30rem and below | 1.15 | -0.022em | 600 | `--type-display`, `-lh`, `-ls`, `-wt` | Page title (`h1`, `.t-display`) |
| headline | 1.25rem (20) | 1.2 | -0.015em | 600 | `--type-headline*` | Section heading (`h2`, `.t-headline`) |
| title | 1.0625rem (17) | 1.3 | -0.01em | 600 | `--type-title*` | Panel and group heading, dialog title (`h3`, `.panel__title`, `legend`) |
| body | 0.9375rem (15) | 1.5 | 0 | 400 | `--type-body*` | Prose, banners |
| dense | 0.8125rem (13) | 1.4 | 0 | 400 | `--type-dense*` | Tables, controls, lists, form labels |
| caption | 0.75rem (12) | 1.35 | +0.01em | 500 | `--type-caption*` | Labels, helper text, axis text, badges |
| metric | 1.75rem (28) | 1.1 | -0.02em | 600 | `--type-metric*` | Large single values, tabular |
| mono | 0.8125rem (13) | 1.4 | 0 | 400 | `--type-mono*` | Ids, hashes, code |

Uppercase labels use `--tracking-label` (0.04em). Minimum size 12 px everywhere in the styles (SimDemo's inline styles are outside this and belong to increment I3). Utility classes: `.t-display .t-headline .t-title .t-body .t-dense .t-caption .t-metric .t-mono .t-upper`, `.prose` (70ch, 15/1.6).

**Deprecated aliases (kept until I4 finishes):** `--text-xs` = caption, `--text-sm` = dense, `--text-md` = body, `--text-xl` = headline, `--text-metric` = metric, `--leading-tight` 1.25, `--leading-body` = body leading. `--text-lg` and `--space-7` were removed (unused).

## 4. Spacing, radius, elevation

| Group | Values |
|---|---|
| Spacing (4 px grid) | `--space-1` 4 to `--space-6` 32 px; semantic: `--space-label` 8, `--space-group` 16, `--space-section` 24 px; `--gutter` 16 px |
| Radius | `--radius-sm` 4 (badges, inputs), `--radius-md` 8 (panels, buttons), `--radius-lg` 12 (dialogs only), `--radius-pill` |
| Targets | `--target-min` 24, `--target-desktop` 32, `--target` 36, `--target-touch` 44 px |

**Elevation (solid surfaces; project override of S12: no glass, no blur):**

| Level | Surface | Used by (classes) |
|---|---|---|
| L0 | `--bg` | page |
| L1 | `--surface`, 1 px `--border`, `--shadow-1` (light only; transparent in dark) | `.panel`, tables, forms |
| L2 | `--surface-raised`, border, `--shadow-2` | `.menu`, `.popover`, `.elev-2`, `.sticky-bar`, `.form-bar` |
| L3 | `--surface-overlay`, border, `--shadow-3`, over `.scrim` (`--scrim`) | `.dialog` |

A panel inside a panel renders as a ruled section (`.panel .panel`); `.panel--plain` is a heading-plus-whitespace section with no box. Shadows only come from `--shadow-1..3` (a test enforces this). `prefers-contrast: more` sets them to `none`.

## 5. Motion and interaction (S1, S3, S4, S7, S11, S14)

| Token | Value | Use |
|---|---|---|
| `--dur-press` | 100 ms | pointer-down feedback |
| `--dur-fast` | 150 ms | hover and colour changes |
| `--dur-base` | 200 ms | disclosure, appear, cross-fade |
| `--dur-slow` | 300 ms | maximum; reserved for panel or sheet movement |
| `--ease-out` | cubic-bezier(0.2, 0.8, 0.2, 1) | default; no springs, no bounce |

- **Press:** `.btn:active` scales to 0.97 (100 ms ease-out) and tints; rows, tabs, nav items, segmented buttons and choices use `--tint-press` only (scale on dense rows reads as jitter). Hover is `--tint-hover`. Focus is the 2 px outline.
- **Disabled:** `.btn:disabled` or `.btn[aria-disabled="true"]`: lower contrast, solid border, `not-allowed`; the reason is shown with `<DisabledReason>` and linked by `aria-describedby`. **Busy:** `aria-busy="true"` plus a spinner glyph and text.
- **Loops:** `.spinner` and `.progress-indeterminate` are the only infinite animations. **Reduced motion:** scale removed, loops static (static striped bar), appearing content cross-fades by opacity (`pc-fade`), disclosure chevron does not rotate smoothly.

## 6. Borders (spec section 5)

Solid 1 px borders for structure. **Dashed and dotted borders are reserved for provenance badges** (`.prov--simulated`, `.prov--replayed`, `.prov--predicted` dashed; `.prov--unavailable` dotted). No double borders (simulated banner and frame use a solid border with a heavy leading or top rule). A test enforces this for `tokens.css` and `app.css`; `lab.css` still has four leftovers owned by increment I2 (`.btn:disabled`, `.lab-tag`, `.lab-effect__zero`, `.lab-unavail-panel`), bounded by the test.

## 7. Provenance badge (spec section 11)

Anatomy: glyph + word (+ short qualifier), caption tier, uppercase, 1 px border; the DOM contract (`prov prov--<key>`, `data-provenance`, `data-tier`) is unchanged.

| Displayed | `data-provenance` | Border | Glyph | Notes |
|---|---|---|---|---|
| REAL (+ basis qualifier) | `measured` | solid | filled circle | never implies more than its basis, for example "loopback" |
| DERIVED | `derived` | solid | filled diamond | |
| DERIVED + "inferred" | `inferred` | solid | outlined diamond | same hue as derived |
| SIMULATED | `simulated` | dashed | triangle | |
| REPLAYED | `replayed` | dashed | double chevron | reserved: rendered only when passed; telemetry cannot produce it (`readField` rejects it) |
| PREDICTED | `predicted` | dashed | dotted ring | reserved; no predictive data exists and the PIE is a stub |
| UNAVAILABLE | `unavailable` | dotted | circle with slash | never a number |

The display words live in `ProvenanceBadge.jsx` (`DISPLAY`). `lib/provenance.js` still names the labels "Measured" and "Inferred"; see the increment record hand-offs.

## 8. State grammar (spec section 10)

`StateBlock` kinds: loading, empty, error, success, warning, disabled, stale, unavailable, simulated (plus disconnected, notEnabled). Each has a glyph and words; error is `role="alert"`; loading is `aria-busy`; the left rule colour follows the kind. Helpers: `Spinner`, `DisabledReason`, `StaleTag`, `InlineError`, `InlineSuccess`. Forms: `.form`, `.field-group` (legend, ruled, no box), `.field` (`.field__label/__hint/__row/__unit/__error`, error hidden until `data-touched`), `.form-summary`, `.choice-list`, `.choice`, `.form-bar` (sticky L2). Charts: `ChartFrame` (`.chart-frame*`, `.seg`).

## 9. Other preferences

`prefers-contrast: more`: strong borders, no shadows. `prefers-reduced-transparency: reduce`: the scrim is more opaque (no other translucency exists). `forced-colors: active`: system-colour borders (badge border style kept), `Highlight` outline for the selected nav item, tab and segmented button, `GrayText` for disabled buttons.

## 10. Contrast

`node scripts/check-contrast.mjs [--markdown]`, run 2026-10-30: **196 of 196 checks pass** (98 pairs times 2 themes). Required: text 4.5:1, UI and non-text 3:1. Lowest margins: text 4.53:1 (`--text-3` on `--surface-2` with the pressed tint, light), non-text 3.66:1 (dark) and 3.85:1 (light, `--border-strong` on `--surface-2`).

Covered pairs: `--text`, `--text-2`, `--text-3` on `--bg`, `--surface`, `--surface-2`, `--surface-raised`, `--surface-overlay`; the same three on `--surface`, `--surface-2`, `--surface-raised` with `--tint-hover` and `--tint-press` blended over them; `--accent` on four surfaces; `--accent-contrast` on `--accent` plain and tinted; each status colour on its `-bg`, on `--surface` and `--surface-2`, and `--text` on each `-bg`; each of the six provenance colours on its own `-bg` and on `--surface` and `--surface-2` (4.5:1) and against `--bg` and `--surface-raised` (3:1, the 1 px border is the same colour); `--border-strong` and `--focus` against all five surface levels (3:1); `--chart-line` on `--surface` and `--surface-2` (3:1).

**Not covered:** decorative `--border` and `--chart-grid`; text over anything other than the listed solid surfaces; the scrim; anything in `SimDemo.jsx`; a real browser render. A passing script is a regression guard for token pairs, not proof that a page is legible.
