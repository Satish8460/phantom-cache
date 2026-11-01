# Design-engineering toolkit research: coordination board

**Status:** Active (lead-owned). **Audience:** agents R1 to R4, the lead, the owner. **Last reviewed:** 2026-10-30. **Authority:** owner directives of 2026-10-30 (multi-skill design-engineering toolkit; R4 system-object research). **All work is research only:** no skill or dependency installation, no source or backend changes, no frontend redesign, no changes to `.claude/skills`.

## Agents and file ownership (one writer per file)
| Agent | Scope | Owned files |
|---|---|---|
| R1 | Design intelligence, art direction, redesign, UI/UX research, design-system generation, critique and audits | `r1_design_intelligence_and_critique.md`, `coord/r1.md` |
| R2 | Materials/glass, motion, animation engineering, 3D/spatial, diagrams, data visualisation | `r2_motion_materials_diagrams_dataviz.md`, `coord/r2.md` |
| R3 | React/frontend engineering, component architecture, accessibility, browser inspection, visual regression, performance | `r3_engineering_a11y_testing.md`, `coord/r3.md` |
| R4 | PhantomCache system objects, lifecycles, operations, telemetry/event availability, motion and continuity opportunities | `r4_system_objects_and_motion.md`, `r4_lifecycle_and_event_matrices.md`, `coord/r4.md` |
| R5 | Systems integration and product-flow supervisor: journeys, device/viewport/theme/state coverage matrix, cross-agent supervision, product-wide consistency, regression register | `r5_product_flow_integration.md`, `r5_coverage_matrix.md`, `coord/r5.md` |
| Lead | Coordination, conflict resolution, integration into the toolkit proposal | `00_coordination.md` (this file), `toolkit_proposal.md` |
| DD (design direction) and Track 2 (accessibility) | Inputs to the final synthesis | `docs/design/phantomcache_design_direction_v2.md`, `docs/design/remediation/03_validation_accessibility.md` (read only for R1 to R4) |

## How agents coordinate
1. Each agent keeps its own `coord/rN.md` (create it first): **current investigation, findings so far (with source references), open questions, dependencies on other agents, conflicting recommendations, decisions requiring coordination, completion status.** Update it as research proceeds, not only at the end.
2. Each agent READS the other `coord/*.md` files and the other agents' main files as they appear, at least before finalising, and answers requests addressed to it inside its own file under a heading `Replies to R<n>`. Requests are written in the requester's file under `Requests to R<n>` with: what is asked, why, what is assumed if unanswered.
3. R4 supplies product facts: object definitions, states, transitions, events and data availability (REAL / DERIVED / SIMULATED / PROPOSED / UNAVAILABLE) with source references. R1, R2 and R3 evaluate external capabilities against R4's actual objects and flows, and say which opportunities are technically meaningful, which need telemetry the backend does not expose, and what it costs to implement and validate.
4. A research recommendation never becomes an implementation decision by itself: record it as PROPOSAL in the proposing agent's file; the lead decides and records it in this board.
5. Disagreements are written down (positions, evidence) in the coord files; the lead resolves them in the section below.

## Shared vocabulary
Provenance labels: REAL (with qualifiers), DERIVED, SIMULATED, REPLAYED, PREDICTED (only where the backend supplies it), UNAVAILABLE. For R4's inventory the implementation status vocabulary is REAL / DERIVED / SIMULATED / PROPOSED / UNAVAILABLE (a design-time object may be PROPOSED or UNAVAILABLE). Product terms follow `docs/glossary.md`.

## Lead decisions and conflict log
2026-10-30: R5 added (supervision function). R5 does not replace specialists, does not implement, and escalates architecture, backend, security, dependency and product-direction decisions to the lead/owner. Requests and replies between agents are tracked in each agent's coord file; the lead copies open items here.

## Status board
| Agent | Status |
|---|---|
| R1 | running |
| R2 | running |
| R3 | running |
| R4 | running |
| R5 | starting (supervises R1 to R4; reviews their files in at least three passes) |
