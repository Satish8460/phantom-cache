> Notice (2026-10-30): Historical work-package specification from the first (simulation-era) phase. "WP1" to "WP5" here are not the "WP-nn" work packages of `docs/research/work_packages.md`. The plan describes intended work, not verified current behaviour: for example, PIE canary rollouts and Count-Min sketch estimation are described here, but `src/control/pie.rs` is a legacy threshold stub (`pie: None` at `src/main.rs:433`) and `CountMinSketch` is used only by a unit test (`tests/unit/wp2_tests.rs`). Current status and evidence: [docs/STATUS.md](../STATUS.md), [handoff CORRECTIONS](../handoff/CORRECTIONS.md).
>
> Original text below is unchanged.

# WP3 Guide — Distributed Failure & Verification

This guide explains how to execute and verify the distributed simulation modules implemented during WP3.

---

## 1. Running Unit & Integration Tests

To run the complete test suite verifying routing determinism, node exclusion states, and replica selection loops:

```bash
cargo test --package phantom-cache-demo --lib -- tests::
```

### Expected Output
```text
running 8 tests
test sim::clock::tests::test_virtual_clock ... ok
test sim::rng::tests::test_seeded_rng_determinism ... ok
test core::cache::tests::test_partitioned_cache_ops ... ok
test core::coalesce::tests::test_coalescer_single_flight ... ok
test dist::node::tests::test_normal_routing ... ok
test dist::node::tests::test_node_failure_exclusion ... ok
test dist::node::tests::test_replica_selection ... ok
test intel::sketch::tests::test_sketch_error_bounds ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## 2. Invariant Verification Checklist

* [x] **Failed node receives no traffic:** Verified via `test_node_failure_exclusion` (where offline node 2 is bypassed for routing).
* [x] **Weighted selection is deterministic:** Rendezvous Hash returns the identical primary target for any seed key hash.
