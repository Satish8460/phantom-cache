# Source and provenance: apple-design skill

| Field | Value |
|---|---|
| Original repository | https://github.com/emilkowalski/skills |
| Skill name | `apple-design` (path in repository: `skills/apple-design/SKILL.md`) |
| Pinned commit | `a81dbca07f2217b7103e510c0f13f8d5b017afc9` (2026-10-28T18:25:28Z, "add remove-animations skill") |
| Licence | MIT, "Copyright (c) 2026 Emil Kowalski" (repository `LICENSE`, 1070 bytes) |
| Adoption date | 2026-10-30 |
| Adopted by | Lead agent, under owner decision O8-1 (APPROVED by the owner on 2026-10-30; the owner asked not to be asked again) |

## Files copied
| File here | Source at the pinned commit | Git blob SHA (verified) | SHA-256 of the file |
|---|---|---|---|
| `SKILL.md` | `skills/apple-design/SKILL.md` | `adf68c517f2d7d2781587d619de07f4de286da57` | `77bb63b7043bb93aca2ff4ab040c249484eef35682bb6ff7163433c31d30adc7` |
| `LICENSE` | `LICENSE` | `57b46f1fd11dc62351ea548104a88aa3f659c11b` | `4ff5bdb7887ec1435c9cab0e8d1a7caee704d894d65c2a008ccc68b1cc2f260b` |

Both blobs were downloaded from `raw.githubusercontent.com` at the pinned commit and their git blob hashes were recomputed locally and matched the GitHub API values. The repository tree has no scripts for this skill (only `SKILL.md`).

## Modifications
**None.** `SKILL.md` and `LICENSE` are byte-identical to the pinned commit. PhantomCache-specific constraints are kept in a separate overlay, `docs/design/apple_design_skill_project_rules.md`, so the original stays verifiable.

## Not adopted
`dickwu/apple-design-skill` and `ykswang/apple-hig-skill` are NOT installed (owner instruction; no licence file in either repository; both embed Apple text). No Apple assets, fonts or branding are copied.

## Notes for users of this skill
- The skill contains an "Initial Response" block: when invoked without a specific question it replies only with a fixed sentence. Invoke it with a concrete question or task.
- Section 12 (translucent materials, `backdrop-filter`) and the spring examples (Motion/Framer Motion) conflict with PhantomCache rules (no glass/blur surfaces as a default, no new dependencies). See the project rules overlay.
- Apple's Human Interface Guidelines and WCAG 2.2 are used as public references only.
