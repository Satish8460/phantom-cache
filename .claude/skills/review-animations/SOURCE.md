# Source and provenance: review-animations skill

| Field | Value |
|---|---|
| Original repository | https://github.com/emilkowalski/skills |
| Skill name | `review-animations` (path in repository: `skills/review-animations/`) |
| Pinned commit | `a81dbca07f2217b7103e510c0f13f8d5b017afc9` (2026-10-28T18:25:28Z, "add remove-animations skill"), the same commit as the installed `apple-design` skill |
| Licence | MIT, "Copyright (c) 2026 Emil Kowalski" (repository `LICENSE`, 1070 bytes) |
| Adoption date | 2026-10-30 |
| Adopted by | Lead agent, under the owner's toolkit adoption approval of 2026-10-30 (docs/research/design_toolkit/toolkit_proposal.md, section 11.1) |

## Files copied
| File here | Source at the pinned commit | Git blob SHA (recomputed from the pinned object) | SHA-256 of the file |
|---|---|---|---|
| `SKILL.md` | `skills/review-animations/SKILL.md` | `577a64397267ce20960de2202950594d5bd6f9b6` | `6ff590d29e436766135aa3d137be1fac2a867165c7257fc315b189361f1873af` |
| `STANDARDS.md` | `skills/review-animations/STANDARDS.md` | `863ea12ea94c1f7a47b5f7e1019e976aa82bd4cd` | `e7d3605034acda54ca13e43aec9e64d65b53de20f75b11b8d694e373012fbe07` |
| `LICENSE` | `LICENSE` | `57b46f1fd11dc62351ea548104a88aa3f659c11b` | `4ff5bdb7887ec1435c9cab0e8d1a7caee704d894d65c2a008ccc68b1cc2f260b` |

Method: content-addressed export with `git show <pinned-commit>:<path>` from a temporary partial clone outside the repository; every file was read in full before copying; no network, script, hook, install or settings change was found in the files.

## Modifications
**None.** The files are byte-identical to the pinned commit. Project constraints live in `docs/design/external_design_skills_project_rules.md` so the originals stay verifiable.

## Notes for users of this skill
- The skill opens with an "Initial Response" block (reply only with a fixed sentence when invoked without a question). Always invoke it with a concrete task; project rules override that directive once a task is given.
- Output formats mandated by the skill (tables, Before/After) are optional; the project finding format applies.
- The skill refers to other Emil skills (`animate`, `improve-animations`) that are NOT installed.
- It is a reviewer or advisor: findings go to the lead; it does not replace DD v2, the design-system spec or `apple-design`.
