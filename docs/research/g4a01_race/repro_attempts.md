# Repro attempts (sandbox D:\Temp\sbx_race; target D:\Personal\_sbx_race_target; cargo -j 1 via with_cargo_lock.ps1, label RACE; --offline)

Sandbox copy: Cargo.toml/Cargo.lock/src/tests/benches + docs/demo/scenario_b_interpretation_notice.md (needed by include_str!). Sandbox-only changes: `sbx_hook` field on PartitionedCache called after the byte update at the :1517 site and before the re-read; new test file (copy: sandbox_g4a01_race_tests.rs.txt in this folder).

Run 1 (unfixed code, hook active), debug profile, 2026-10-30:
- control_invalidate_before_check_is_fenced: ok (invalidate after begin_fetch, before finalize: body rejected).
- g4a01_tenant_invalidate_between_check_and_restamp: FAILED, stale body "PRE" returned by get_resource after invalidate_tenant had returned (invoked from a second thread while shard mutex held).
- g4a01_global_invalidate_between_check_and_restamp: FAILED, same with invalidate_all.
- g4a01_stress_no_hook: 20 s, 3 threads (filler, invalidator, reader), tagged bodies: 3,160,958 fills, 0 violations.

Run 2 (fix applied in sandbox, hook still present): all 4 tests ok; stress 2,647,107 fills, 0 violations.

Limitations:
- Confirmation relies on an injected yield point: it proves the interleaving is semantically possible and harmful, not its frequency. Real-schedule probability unmeasured.
- Stress oracle is conservative (tag <= acked captured before invalidate) and a single key; debug build; one run each; Windows loopback-free in-process only. Zero hits do not bound the probability.
- Single run per configuration, no confidence intervals; not a performance claim.
- No loom/model checking run.
