//! Phantom Lab: reusable experiment runner (D-3).
//!
//! Tier-1 (T1) deterministic virtual-time simulation that drives the REAL product components
//! (`core::cache`, `core::coalesce`, `intelligence::executor`, candidates) against a simulated
//! origin, plus seeded workload generation, event queue, raw result serialization, and the
//! paired statistical analysis used by the M6 protocol.
//!
//! Every number produced here is tier SIMULATED. Real-process (T2) helpers (gateway, load
//! generator, origin process) remain in `tests/support/lab` because they need test-only crates.
//!
//! Entry points: [`runner::execute`] (library API with `RunSpec`/`RunResult`) and the
//! `phantom-lab` binary. The matrix driver [`eval::run_matrix`] is shared with the test suite.

#![allow(dead_code)]

pub mod analysis;
pub mod clock;
pub mod eval;
pub mod grid_tuning;
pub mod m6_pipeline;
pub mod events;
pub mod eviction_baselines;
pub mod manifest;
pub mod matrix_runner;
pub mod origin_sim;
pub mod report_gen;
pub mod rng;
pub mod runner;
pub mod t1_sim;
pub mod tuning;
pub mod workloads;
