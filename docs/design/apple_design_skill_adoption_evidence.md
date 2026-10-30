# apple-design skill: adoption and invocation evidence (O8-1)

**Status:** Active. **Audience:** product owner, UI/UX agents. **Last reviewed:** 2026-10-30. **Authority:** owner decision O8-1 (APPROVED 2026-10-30). Related: `.claude/skills/apple-design/SOURCE.md`, `docs/design/apple_design_skill_project_rules.md`.

## 1. What was adopted (FACT, verified by the lead)
- Repository `https://github.com/emilkowalski/skills`, HEAD at verification time `a81dbca07f2217b7103e510c0f13f8d5b017afc9` (2026-10-28T18:25:28Z), the same commit the Phase 8 package and the P4 audit recorded. GitHub API licence endpoint: MIT, `LICENSE` 1,070 bytes.
- Skill `skills/apple-design/SKILL.md` (23,036 bytes) and `LICENSE`, downloaded from `raw.githubusercontent.com` at the pinned commit. Local `git hash-object` equals the GitHub blob SHAs (`adf68c51...` and `57b46f1f...`). SHA-256 values are in `.claude/skills/apple-design/SOURCE.md`.
- Installed byte-identical at `.claude/skills/apple-design/` (project skill structure) with `LICENSE` and `SOURCE.md`. No modification.
- The contents match the earlier audit (A3, P4): same licence, same commit, one skill file, no scripts. No discrepancy, so adoption continued.
- NOT installed: `dickwu/apple-design-skill`, `ykswang/apple-hig-skill`. No Apple assets, fonts or branding copied.

## 2. Discoverability and invocation (FACT, with limits)
| Test | Result |
|---|---|
| Invoke `apple-design` through the Skill tool in the lead's running session | **Failed:** `Unknown skill: apple-design`. The skill list is read when a session starts, and `.claude/skills/` did not exist before today. The running session (and the sub-agents it starts) cannot see the skill until Claude Code is restarted. |
| Start a fresh Claude Code process in the project directory (`claude.exe -p`, `--allowedTools Skill`, session id `6090c5b2-e09e-4b9b-b9a1-64adaa47c644`, 2026-10-30 about 12:35) and ask it to invoke `apple-design` with a concrete question | **Succeeded.** The Skill tool call returned the skill body; the process answered from it (press feedback `scale(0.97)` over 100 ms on pointer-down; system font, size-specific tracking and leading; critically damped motion, 0.3 to 0.4 s response; reduced-motion cross-fades, `prefers-reduced-transparency`, `prefers-contrast`; skip momentum, springs, haptics and translucent materials for dense data views). No permission denials; cost reported as 0.185 USD. |

| Update, same day, about 12:55: the running lead session's skill list refreshed and listed `apple-design`; the lead then invoked it through the Skill tool with a concrete task (the design-system foundation spec) | **Succeeded in the running session.** The tool reported "Launching skill: apple-design" with base directory `d:\Personal\phantom-cache\.claude\skills\apple-design` (the project copy) and returned the full skill body. |

The skill therefore **is discoverable in this project and was actually invoked twice through the supported mechanism: once by a fresh process, once by the lead in the running session** (the first lead attempt at about 12:31 had failed with `Unknown skill` because the list had not refreshed yet). The fresh process quoted the skill's "Initial Response" rule correctly and bypassed it by asking a concrete question. Note: the `Unknown skill` result and the later success are both facts; the skill list in a running session may refresh some time after the directory is created.

## 3. How agents in the current session use it
- INFERENCE and limit (UNVERIFIED after the 12:55 refresh): sub-agents may or may not receive the refreshed skill list; agents started before the refresh (B0, RC, DR, AUDIT) had no way to invoke it. Treat sub-agent invocation as unproven until an agent's report shows a successful Skill-tool call. For this reason every design agent brief in this work package requires the agent to read `.claude/skills/apple-design/SKILL.md` in full, to cite the section number (S1 to S17) behind each design decision, and to fill the compliance table in `docs/design/apple_design_skill_project_rules.md`. That is "reading the installed pinned skill", not a Skill-tool invocation, and reports must say so.
- RECOMMENDATION: restart the Claude Code session (or open a new one) so the skill appears in the skill list; all later sessions can then invoke it directly.
- A skill read is not proof of good design: see the validation tracks in the UI/UX remediation plan.
