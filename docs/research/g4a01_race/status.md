# G4A-01 status (RACE agent, AI second look, 2026-10-30)

Classification: CONFIRMED (deterministic interleaving, sandbox-injected yield point); natural-schedule reproduction NOT achieved.

- Reproduced here (sandbox, debug build, Windows): with a sandbox-only hook between the fence check and the epoch re-read in `finalize_fetch`, an acknowledged `invalidate_tenant` (and `invalidate_all`) leaves a pre-invalidation body readable via `get_resource`. Two failing tests; control test (invalidation before the check) is correctly fenced.
- Not reproduced: 20 s unhooked 3-thread stress (3,160,958 fills, 0 violations). The window is nanoseconds wide; absence is not evidence of unreachability (see analysis.md).
- Reachability argument from code: the shard mutex is held across both reads, but the epoch bump never takes the shard mutex, so nothing orders it against the window. Lock order shard -> tenant_epochs is not violated by the bug; finalize never takes tenant_epochs at all.
- Proposal (sandbox only, NOT applied to the main tree): stamp the epochs captured by the fence check. With it all 4 sandbox tests pass. See proposal.patch.
- Main tree: no src/tests/Cargo changes made by this agent.
