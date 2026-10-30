> Notice (2026-10-30): Historical work-package specification from the first (simulation-era) phase. "WP1" to "WP5" here are not the "WP-nn" work packages of `docs/research/work_packages.md`. The plan describes intended work, not verified current behaviour: for example, PIE canary rollouts and Count-Min sketch estimation are described here, but `src/control/pie.rs` is a legacy threshold stub (`pie: None` at `src/main.rs:433`) and `CountMinSketch` is used only by a unit test (`tests/unit/wp2_tests.rs`). Current status and evidence: [docs/STATUS.md](../STATUS.md), [handoff CORRECTIONS](../handoff/CORRECTIONS.md).
>
> Original text below is unchanged.

# WP1 — Real Phantom + Simulation Core

## Objective
Establish the Rust project codebase, virtual clock, RNG scheduler, and the local Cache Core (partitioned mutex buckets, request coalescing, and baseline eviction).

## Scope
* System clock and virtual Clock.
* Seeded RNG source.
* Poisson / Zipfian traffic generator.
* Network latency & database capacity queue models.
* Partitioned Cache using sharded Mutex buckets.
* Single-flight request coalescer with tenant/authorization parameters.

## Architecture
```text
[Traffic Generator] ──► [Network Link] ──► [Coalesced Cache Core] ──► [Database Queue]
```

## Real vs Simulated Boundary
* **Real:** Partitioned Mutex cache structure, visited-sweep evictions (SIEVE), authorization request coalescing.
* **Simulated:** Network latency, packet drops, M/M/1 database capacity saturation.

## Files Created
* `src/main.rs`
* `src/sim/clock.rs`
* `src/sim/rng.rs`
* `src/sim/scheduler.rs`
* `src/sim/traffic.rs`
* `src/sim/network.rs`
* `src/sim/backend.rs`
* `src/core/cache.rs`
* `src/core/coalesce.rs`

## Completion Criteria
* Rust project initialized.
* Simulation modules compilation without errors.
* Core unit tests successfully pass.

## Status
* **COMPLETED**
