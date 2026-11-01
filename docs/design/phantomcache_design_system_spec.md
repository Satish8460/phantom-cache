# PhantomCache design-system specification (foundation)

**Status:** Proposed (lead draft; becomes Active when the owner or the independent review accepts it as the single shared specification; until O8-2 it governs implementation work only). **Audience:** UI/UX implementation agents, reviewers, the owner. **Last reviewed:** 2026-10-30. **Authority:** owner authorization of 2026-10-30 (sections 4, 5 and 5.1); skill `.claude/skills/apple-design` (pinned, MIT; SOURCE.md); project rules `docs/design/apple_design_skill_project_rules.md`; audit `docs/design/remediation/02_ui_audit_and_plan.md`. Supersedes nothing; extends `docs/design/tokens.md` and `principles.md`, which stay as history.

**Rule:** this document is the only definition of shared tokens and component behaviour. Only the increment-I1 owner edits `ui/src/styles/tokens.css`, `app.css` and the shared components. Everyone else consumes tokens; a missing token is requested from the lead, never invented locally. Skill sections are cited as S1 to S17.

## 0. Principles for this product (what Apple-inspired means here)
A precise, restrained engineering product: hierarchy comes from type, spacing and grouping, not from decoration (S15, S16 Simplicity not minimalism, Craft). Not everything is a card. Dense technical data stays dense where it is needed; secondary detail moves one level deeper (S16). Motion confirms action and never decorates (S1, S13 utility). Every value shows its provenance (project override: non-negotiable).

## 1. Typography (S15)
System font stack only (no bundled or Apple fonts). Sizes in `rem`; tracking and leading change with size; hierarchy from size plus weight plus leading together; minimum 12 px; numerals tabular wherever values align.

| Token | Size | Leading | Tracking | Weight | Use |
|---|---|---|---|---|---|
| `--type-display` | 1.75rem (28) | 1.15 | -0.022em | 600 | Page title (one per page) |
| `--type-headline` | 1.25rem (20) | 1.2 | -0.015em | 600 | Section heading (h2) |
| `--type-title` | 1.0625rem (17) | 1.3 | -0.01em | 600 | Panel / group heading (h3), dialog title |
| `--type-body` | 0.9375rem (15) | 1.5 | 0 | 400 | Prose, descriptions |
| `--type-dense` | 0.8125rem (13) | 1.4 | 0 | 400 | Tables, form controls, lists (default for data) |
| `--type-caption` | 0.75rem (12) | 1.35 | +0.01em | 500 | Labels, captions, badges, axis labels |
| `--type-metric` | 1.75rem (28) | 1.1 | -0.02em | 600, `tabular-nums` | Large single values |
| `--type-mono` | 0.8125rem (13) | 1.4 | 0 | 400, `tabular-nums` | Ids, hashes, code, numeric columns |

At 390 px the display tier steps down to 1.5rem. Long ids and hashes wrap (`overflow-wrap: anywhere`) inside their container. Old `--text-*` aliases stay as deprecated aliases until I4 finishes.

## 2. Spacing, layout, radii
4 px grid via `--space-1..7` (unchanged). Page content max width 72rem; gutters 1rem (phone) / 1.5rem (>=820). Group spacing: 1.5rem between sections, 1rem inside a panel, 0.5rem between a label and its field. Radii: `--radius-sm` 4 px (badges, inputs), `--radius-md` 8 px (panels, buttons), `--radius-lg` 12 px (dialogs only). Page structure: header (title, one-line purpose, ONE primary action) then content; at most **two** banner rows above the primary task at 1280x800 (rest collapses into a status strip, section 8).

## 3. Colour, themes, contrast
Single source for both themes (`color-scheme` plus `light-dark()`, with the existing data-theme override and OS preference preserved); the duplicated dark block is removed. Existing palette values stay unless a contrast check fails. Required contrast: 4.5:1 for text, 3:1 for UI component boundaries and non-text meaning (icons, chart lines, focus ring), checked by `npm run check:contrast` on solid surfaces. Colour never carries meaning alone (icon plus text plus border pattern). `prefers-contrast: more` raises borders to `--border-strong` and removes shadows (S14). Forced-colors: system colours, badges keep their text and glyph.

## 4. Elevation and surfaces (solid; project override of S12)
| Level | Token | Surfaces | Treatment |
|---|---|---|---|
| L0 | `--bg` | Page | flat |
| L1 | `--surface` + `--shadow-1` + 1 px `--border` | Panels, tables, forms | resting; light theme uses border plus a faint shadow; dark theme uses a lighter surface step plus border, no shadow |
| L2 | `--surface` + `--shadow-2` + border | Menus, popovers, sticky run header, sticky action bar, mobile nav sheet | floating but opaque |
| L3 | `--surface` + `--shadow-3` + scrim `rgb(0 0 0 / .4)` | Dialogs (cancel confirmation, pairing) | modal; background dims (S12 dim to focus) |

Rules: no nested panels (a panel never sits inside a panel; group with headings and spacing); no glass/blur; shadows only from these tokens. A panel needs a reason: prefer headings plus whitespace for grouping.

## 5. Borders
Solid 1 px borders for structure. **Dashed and dotted borders are reserved for provenance badges and nothing else.** No double borders. Focus uses the outline token only.

## 6. Motion and interaction feedback (S1, S3, S4, S7, S14)
| Token | Value | Use |
|---|---|---|
| `--dur-press` | 100ms | pointer-down feedback |
| `--dur-fast` | 150ms | hover, colour changes |
| `--dur-base` | 200ms | disclosure, appear/disappear, cross-fade |
| `--dur-slow` | 300ms | panel/sheet movement (maximum) |
| `--ease-out` | cubic-bezier(0.2, 0.8, 0.2, 1) | default |

- **Press (S1):** on pointer-down buttons scale to 0.97 (`transition: transform 100ms ease-out`); table rows, list items and tabs use a background tint instead of scale (scale on dense rows reads as jitter). Feedback never waits for `click`.
- **Hover** is a tint only; **focus-visible** a 2 px outline, 2 px offset; **disabled** lower contrast plus a stated reason (section 9); **busy** a spinner glyph plus text, `aria-busy`.
- Never lock input during a transition (S3). Enter and exit along the same path; popovers originate from the trigger (S7). Only `transform` and `opacity` animate (S11).
- No springs, no bounce, no looping decoration (S4 default damping 1.0). The indeterminate progress indicator is the only loop and becomes static under reduced motion.
- **Reduced motion (S14):** `prefers-reduced-motion: reduce` replaces any movement with a 200 ms opacity cross-fade; press scale is removed (tint remains).

## 7. Controls and components
Buttons: primary (one per view), secondary, ghost, destructive; sizes 32 px desktop / 44 px touch minimum height; never below 24 px. Inputs 36 px high (44 at touch). Checkboxes and radios use a 24 px control inside a full-row label (32/44 px). Tables: dense tier, sticky header, tabular numerals, scroll inside their own container, caption or aria-label; row tint on hover; no zebra unless it aids scanning. Tabs and segmented controls: label plus selected state not by colour alone. Dialogs: L3, focus trap, Escape closes and returns focus, destructive actions confirmed once with specific wording (S16 Agency). Charts: `ChartFrame` with title, provenance badge, axis captions at full width, units, and a Chart/Table toggle (table alternative always available). Icons: one stroke family, 16 px default, always paired with text or an accessible name.

## 8. Information architecture patterns
Wayfinding (S16): every page answers where am I, where can I go, what is here. Side navigation from 820 px, menu button below; groups named for contents; per-route `document.title`. **Status strip:** connection and mode information (gateway, auth, build profile, loopback vs LAN) live in ONE compact strip that expands only when attention is needed; it replaces the always-open Connection panel plus banners. Primary workflow first: on Lab, "Start a run" is the first action; evidence and history follow. Progressive disclosure: advanced options one level deeper.

## 9. Forms
Group related fields in `fieldset` with a title-tier `legend` and a caption-tier helper; place controls next to what they affect (S16 grouping and mapping). Labels are specific. **Validation is by touched state:** no error shows on a pristine form; inline error (icon, text, `aria-describedby`) appears after blur or after a submit attempt; a summary with links to fields appears after a failed submit; success is inline. A disabled primary action states why. Long forms use a sticky action bar (L2) and a short "what happens next" line. Defaults and presets are shown, never hidden.

## 10. State grammar (every async region uses exactly these)
| State | Anatomy |
|---|---|
| Loading | skeleton or inline "Loading <what>" plus `aria-busy`; no spinner without text |
| Empty | what is empty, why, one next action |
| Error | what failed, what it means for the user, retry, and detail disclosure; role alert for blocking errors |
| Success | inline confirmation with the object name; auto-dismiss only for non-critical |
| Warning | before-problem notice with the specific condition |
| Disabled | control stays visible; reason shown |
| Stale | last-update age shown with the value; value keeps its provenance |
| Unavailable | "Unavailable" plus the reason; never zero |
| Simulated | the SIMULATED badge on every value and on the region header |
Feedback kinds status / completion / warning / error map to the neutral, ok, warn and critical tokens with icon and text (S16).

## 11. Provenance presentation
Vocabulary (owner, 2026-10-30): **REAL, SIMULATED, REPLAYED, PREDICTED, DERIVED, UNAVAILABLE.** Mapping from existing labels: MEASURED to **REAL** (always with its basis qualifier, for example "REAL · loopback · single host · debug build"; a three-process run is never drawn as three hosts), INFERRED to **DERIVED** with qualifier "inferred", SIMULATED unchanged, UNAVAILABLE unchanged. REPLAYED and PREDICTED tokens, glyphs and styles are reserved and rendered only when the backend supplies such data (none exists today; the PIE is a stub and must not be activated). Badge anatomy: glyph + label (+ short qualifier), caption tier, 1 px border: solid for REAL and DERIVED, dashed for SIMULATED, PREDICTED and REPLAYED, dotted for UNAVAILABLE; five distinguishable hues plus grey; contrast >= 4.5:1. The DOM contract tested by `ui/test/provenance.test.mjs` stays (class names and data attributes) unless the test is updated in the same change. A comparison between screens never implies a simulated or unavailable capability is real.

## 12. Accessibility baseline (WCAG 2.2 AA)
Keyboard order follows reading order; skip link first; focus never lost after navigation or dialog close; targets 24 px minimum (32 desktop / 44 touch for primary); focus ring 3:1; reduced motion, contrast and forced colors honoured; headings do not skip; live regions for run status (polite) and blocking errors (alert); text can scale to 200% without loss; reflow at 320 px without page-level horizontal scroll. Automated checks are regression guards, not proof of quality.

## 13. Out of scope / not allowed
New dependencies (spring libraries, UI kits, icon packs, chart libraries); glass/blur surfaces; Apple fonts, assets, logos or interface copy; backend changes; decorative animation; hiding a provenance label to look cleaner.

## 14. How compliance is shown
Each increment record (`docs/design/remediation/increment_template.md`) lists spec sections used, skill sections cited and the checks run. Visual quality is judged by the independent review rubric (audit section 5) and by human review, not by tests.
