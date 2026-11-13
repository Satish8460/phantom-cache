//! Phantom Lab live (tier T2, MEASURED_LOOPBACK): real-process experiments on loopback.
//!
//! Scenario B (origin slowdown): the real `phantom-cache-demo` gateway process, a scripted raw-TCP
//! origin with a degradation schedule and an open-loop load generator, driven in real time.
//! Contract: `docs/demo/scenario_B_origin_slowdown.md`. Artifacts: `docs/demo/scenario_b_artifacts.md`.
//! Uses only existing runtime dependencies (tokio, serde); no HTTP client crate (minimal HTTP/1.1 in [`http`]).

pub mod analysis;
pub mod cli;
pub mod gateway;
pub mod http;
pub mod loadgen;
pub mod origin;
pub mod scenario_b;
