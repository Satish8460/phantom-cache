# Design Principles (M8-03)

**Date:** 2026-10-29. **Status:** implemented direction, pending owner approval O8-2 (the owner authorised implementation; formal design approval has not been recorded).

The interface is **inspired by the discipline of Apple's design guidance** (clarity, deference to content, hierarchy, feedback, restrained motion) and is **not** an Apple interface. It uses no Apple assets, SF fonts, SF Symbols, trademarks or copied layouts. Icons are original 16 px SVG shapes in `ui/src/components/Icon.jsx`. Fonts are the platform's own system stack. No external design skill was installed or adopted; this document applies the principles in `docs/research/phase_8_execution_package.md` section 4 and `skill_strategy.md` by hand.

## 1. Honesty first

| Principle | In practice | Test |
|---|---|---|
| Every value says where it came from | A provenance badge (shape and text, never colour alone) on every displayed value: Measured, Derived, Inferred, Simulated, Unavailable. | `render.test.mjs`, `a11y-lint.mjs` rule `provenance` |
| Unavailable is not zero | Unavailable and missing fields render as words with a reason. A genuine measured 0 renders as 0. | `viewmodel.test.mjs` |
| The UI computes nothing new | Only formatting (ratio to percent, bytes to KiB) in `lib/format.js`. Charts plot received values. No rates, hit ratios, costs or counterfactuals. | `forbidden.test.mjs` arithmetic scan |
| Simulation never looks live | Simulated content lives behind one route, under a permanent banner and a double border, and is never mixed into live screens. | `forbidden.test.mjs`, `render.test.mjs` |
| Contract violations are withheld, not shown | A live frame containing a simulated or malformed field has that value hidden and the screen marked Degraded. | `viewmodel.test.mjs` |
| Absence is stated | Screens that need a backend contract that does not exist say so and point to `ui_api_requirements.md`. They never show invented numbers. | placeholder tests |

## 2. Data states are first-class

Every data region can be in exactly one of: **loading** (no frame yet), **live**, **stale** (no frame for 3 s), **disconnected** (stream down), **error** (frames unreadable and none ever read), **degraded** (flowing, with contract violations or an unreadable frame), plus per-value **unavailable / missing / rejected**. Last-known values are kept on screen only when flagged stale with their age. Decision logic is pure and tested (`lib/connection.js`, `lib/viewmodel.js`).

## 3. Visual language

- **Restrained and legible.** Neutral cool greys, one blue accent, no gradients, no blur, no translucency, no glass. Content sits on flat surfaces with 1 px borders; elevation is reserved for floating layers (none exist today).
- **One type scale** (`--text-xs` to `--text-xl`, 12 to 22 px), system font stack, tabular numerals for all figures, monospace only for identifiers and URLs. Minimum text size 12 px.
- **4 px spacing grid**, small radii (4 to 8 px). No oversized rounded cards.
- **Status is never colour alone.** Status and provenance use an icon shape, a text label and, for provenance, a border style (solid, dashed, double, dotted).
- **Both themes** via tokens: OS preference by default, manual override persisted when storage is available. Contrast is checked by script for every pair in all three theme definitions.

## 4. Interaction and motion

- Native elements first (`a`, `button`, `details`, `table`). Keyboard use needs no custom widgets. The side navigation additionally supports arrow keys, Home and End.
- A skip link is the first tab stop. After a route change focus moves to the screen heading.
- Focus rings are always visible (2 px, 3:1 contrast, offset).
- Pointer targets are at least 24 px and 36 px for primary controls.
- **When not to animate:** live values never animate between ticks; nothing animates on keyboard-initiated actions; there are no decorative loops, skeleton shimmer or parallax. The only transitions are 120 ms colour fades on hover, and `prefers-reduced-motion` reduces all durations to near zero. The "loading" icon is static.
- Connection changes are announced once through a polite live region; per-tick updates are not announced.

## 5. Rejected patterns and why

| Rejected | Reason |
|---|---|
| Liquid-glass or blur materials, animated gradients | Reduces contrast and can make data regions look "alive"; no benefit for an operations console. |
| Hero-sized headings, marketing layouts | Wasted space for dense, repeat-use screens. |
| Colour-only status dots | Fails colour-blind users and WCAG 1.4.1. |
| Number tween animations | Implies smoothness the data does not have; hides sampling. |
| Skeleton loaders that look like data | Could be mistaken for values. |
| A chart library | Not authorised (O8-3); hand-rolled SVG is enough for sparklines and bars, each with a "view as table" alternative. |
| Copying iOS or macOS controls, SF Symbols, SF fonts | Licensing and the explicit non-goal. |
| Client-side "savings" or "Phantom OFF" comparisons | Fabricated counterfactuals; moved to the bannered simulation demo only. |

## 6. Accessibility baseline (target WCAG 2.2 AA)

Landmarks (`header`, `nav`, `main`, `footer`), one `h1` per screen, ordered headings, named regions, captioned tables with column headers, labelled SVG, text alternatives for every chart, no information by colour alone, reflow-friendly grids, reduced-motion, forced-colors and higher-contrast hooks. What is verified and what is not is listed in `ui_audit.md` section 5.
