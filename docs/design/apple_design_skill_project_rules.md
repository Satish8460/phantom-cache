# apple-design skill: PhantomCache project rules

**Status:** Active. **Audience:** UI/UX agents and reviewers. **Last reviewed:** 2026-10-30. **Authority:** owner decision O8-1 and the UI/UX remediation authorization (2026-10-30). The skill file `.claude/skills/apple-design/SKILL.md` is a pinned, unmodified copy; this document holds the project's overrides so the copy stays verifiable.

## 1. How to use the skill
1. Read `.claude/skills/apple-design/SKILL.md` before any UI change. Cite the section (S1 to S17) behind each design decision in the change notes.
2. When invoking it through the Skill tool, ask a concrete question (otherwise it answers with one fixed sentence).
3. Treat the skill as a reference for principles, not as a style to imitate. The goal is a precise, restrained engineering product. Do not turn every group of data into a card.

## 2. What applies (adopt)
| Skill section | PhantomCache rule |
|---|---|
| S1 Response | Every control gives instant feedback on pointer-down: `:active` press (scale 0.97, 100 ms ease-out) on buttons; background tint (no scale) on dense rows and table cells. No artificial delays. |
| S3 Interruptibility, S7 spatial consistency | Never lock input during a transition. Enter and exit on the same path. Popovers and menus use the trigger as `transform-origin`. |
| S4 Behaviour over animation | Default critically damped, no overshoot, 300 ms or less. No bounce unless a gesture carried momentum (there is none in the current product). |
| S14 Reduced motion | `prefers-reduced-motion`: cross-fade only (about 200 ms), no slide or scale. Also honour `prefers-contrast` and `prefers-reduced-transparency`. |
| S15 Typography | System font stack first; size-specific tracking (tighter on display sizes, near 0 for body, slightly positive for small labels); leading inverse to size; hierarchy from weight plus size plus leading together; `rem`-based spacing so text scaling does not break layouts; tabular numerals for metrics and tables. |
| S16 Principles | Purpose, Agency (forgiveness; confirm only truly destructive actions), Familiarity and consistency, Simplicity not minimalism (common path first, advanced one level deeper), Craft. Tactical rules: four feedback kinds (status, completion, warning, error), inline validation, wayfinding, grouping and mapping, specific labels. |
| S17 Process | Build interactive prototypes, design interaction and visuals together, review motion in slow motion, test with real people. |

## 3. What does not apply (skip or override)
| Skill section | Rule here | Reason |
|---|---|---|
| S12 Materials (`backdrop-filter`, translucent chrome) | **Not a default.** The design system rejects glass/blur surfaces for data views (legibility, contrast checks are done on solid backgrounds). Allowed only as a proposal reviewed with contrast evidence for a specific non-data overlay (for example a dialog scrim). | Contrast CI and the existing design principles use solid surfaces. |
| S4-S6, S9, S10, S13 (spring libraries, velocity handoff, momentum projection, rubber-banding, haptics and sound) | Skip. If a spring is ever needed, use CSS (`linear()` easing) or the Web Animations API. | No gesture-driven UI exists; **no new dependency** (Motion/Framer Motion) without owner approval. |
| S8 Hinting, S11 frame-level smoothness | Apply only where an interaction actually has a trajectory (panels, drawers). | Avoid decoration. |

## 4. Non-negotiables inherited from the project
- Every displayed value carries its provenance label: REAL, SIMULATED, REPLAYED, PREDICTED, DERIVED or UNAVAILABLE (see `docs/STATUS.md`, `docs/research/vision/three_pillar/04_lab_ux_design.md`). Design must never hide or play down a label to look cleaner. No synthetic data presented as live.
- Honest negative results use the same visual weight as positive ones.
- Dense technical information stays dense where users need it; use progressive disclosure for secondary detail, not removal.
- Accessibility: WCAG 2.2 AA, keyboard order, focus visibility, 24 px minimum target (32 px desktop and 44 px touch for primary controls), reduced motion, forced colors.
- No Apple proprietary assets, fonts, logos or interface copy. Apple's public Human Interface Guidelines and WCAG 2.2 are references only.
- No new npm dependency, no framework change, no backend change under this work. A UI need that requires a backend change is written down as a dependency and sent to the lead.

## 5. Compliance table (agents fill this in their change notes)
| Change | Skill section(s) used | Project override applied | How it was checked |
|---|---|---|---|
| | | | |

## Update (2026-10-30, owner visual addendum and design direction v2)
Section 12 row above ("Not a default") is partly superseded. The owner's visual addendum invites purposeful glass, depth and motion, and `docs/research/design_toolkit/` plus `docs/design/phantomcache_design_direction_v2.md` set the current limits: translucent materials on navigation chrome only (never on data views), measured budgets of at most 4 glass surfaces on desktop and 2 on phone, no 3D tilt, `prefers-reduced-transparency` and contrast checks honoured. Where this overlay and direction v2 differ, direction v2 applies. The other sections of this overlay are unchanged.
