//! Phantom Lab Support Module (M4).
//!
//! Provides deterministic Tier-1 simulation, open-loop Tier-2 harness,
//! workload generators, origin models, static baselines, and manifest validation.

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod baselines;
pub mod clock;
pub mod events;
pub mod eviction_baselines;
pub mod gateway;
pub mod loadgen;
pub mod manifest;
pub mod matrix_runner;
pub mod origin_proc;
pub mod origin_sim;
pub mod report_gen;
pub mod rng;
pub mod t1_sim;
pub mod eval;
pub mod tuning;
pub mod workloads;
pub mod analysis;
