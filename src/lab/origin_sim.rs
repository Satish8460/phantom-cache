//! Simulated origin queueing model and degradation scenarios for Phantom Lab (M4-03 / WP-15).
//!
//! Implements:
//! - M/M/c parallel server queueing model.
//! - Service distributions: Constant, Exponential, Lognormal, Pareto.
//! - Degradation scenario scripts:
//!   - O-NONE: Healthy control
//!   - O-ST: Capacity step-down x r at timestamp t
//!   - O-LAT: Latency inflation x k at timestamp t
//!   - O-BR: Brownout (error rate e during [t1, t2])
//!   - O-OUT: Outage of duration d (HTTP 503)
//!   - O-REC: Slow recovery (capacity ramp)
//!   - O-FL: Flapping (alternating up/down)
//!   - O-HC: Heterogeneous per-key cost
//!   - O-CO: Cold start
//!   - O-RT: Client retry policy with timeout & budget

use super::rng::StreamRng;
use std::collections::BinaryHeap;
use std::cmp::Reverse;

/// Service time distribution for simulated origin backend.
#[derive(Debug, Clone)]
pub enum ServiceDistribution {
    Constant(u64), // us
    Exponential { mean_us: f64 },
    Lognormal { mu: f64, sigma: f64, min_us: u64, max_us: u64 },
    Pareto { scale_us: f64, shape_alpha: f64, max_us: u64 },
}

impl ServiceDistribution {
    pub fn sample(&self, rng: &mut StreamRng) -> u64 {
        match self {
            Self::Constant(us) => *us,
            Self::Exponential { mean_us } => {
                let lambda = 1.0 / mean_us;
                (rng.next_exponential(lambda) as u64).max(1)
            }
            Self::Lognormal { mu, sigma, min_us, max_us } => {
                let u1 = rng.next_f64().max(1e-15);
                let u2 = rng.next_f64();
                let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                let val = (mu + sigma * z).exp() as u64;
                val.clamp(*min_us, *max_us)
            }
            Self::Pareto { scale_us, shape_alpha, max_us } => {
                // Pareto inverse CDF: X = scale / (U^(1/alpha))
                let u = (1.0 - rng.next_f64()).max(1e-15);
                let val = (*scale_us / u.powf(1.0 / shape_alpha)) as u64;
                val.clamp(*scale_us as u64, *max_us)
            }
        }
    }
}

/// Scriptable degradation scenario.
#[derive(Debug, Clone)]
pub enum DegradationScenario {
    None,
    CapacityStepDown {
        at_us: u64,
        factor: f64, // e.g. 0.5 (half capacity)
    },
    LatencyInflation {
        at_us: u64,
        factor: f64, // e.g. 3.0 (3x slower)
    },
    Brownout {
        start_us: u64,
        end_us: u64,
        error_rate: f64, // e.g. 0.20 (20% errors)
    },
    Outage {
        start_us: u64,
        end_us: u64,
    },
    RecoveryRamp {
        start_us: u64,
        end_us: u64,
        initial_capacity_factor: f64, // ramps up to 1.0
    },
    Flapping {
        start_us: u64,
        end_us: u64,
        cycle_us: u64,
        down_duration_us: u64,
    },
    HeterogeneousCost,
    ColdStart,
    /// Capacity is 1.0 before `start_us`, then ramps linearly from `initial_capacity_factor`
    /// up to 1.0 at `end_us` (used after an outage for O-SLW; unlike `RecoveryRamp` it does not
    /// degrade the origin before `start_us`).
    CapacityRecovery {
        start_us: u64,
        end_us: u64,
        initial_capacity_factor: f64,
    },
    /// Latency multiplier 1.0 before `start_us`, rising linearly to `final_factor` at `end_us`,
    /// held afterwards (O-GRD).
    GradualInflation {
        start_us: u64,
        end_us: u64,
        final_factor: f64,
    },
    /// Several regimes active simultaneously: capacity/latency/cost factors multiply, the first
    /// failing regime (in listed order) decides errors.
    Composite(Vec<DegradationScenario>),
}

/// Result of an origin fetch.
#[derive(Debug, Clone)]
pub struct OriginResponse {
    pub start_service_us: u64,
    pub finish_service_us: u64,
    pub queue_wait_us: u64,
    pub is_error: bool,
    pub status_code: u16,
    pub body_bytes: usize,
}

/// Simulated parallel origin model (M/M/c queueing model).
pub struct SimulatedOrigin {
    base_capacity: usize,
    distribution: ServiceDistribution,
    scenario: DegradationScenario,
    server_busy_until: BinaryHeap<Reverse<u64>>, // Earliest time each server becomes free
    // Telemetry counters
    total_dispatched: u64,
    total_completed: u64,
    total_errors: u64,
    peak_in_flight: usize,
    sum_queue_wait_us: u64,
    sum_service_time_us: u64,
    /// Common random numbers for brownout faults (protocol v2.2, finding M2): when set, the fate of a
    /// fetch is a pure function of (seed, key, dispatch-time bucket) and no longer of how many fetches
    /// were dispatched earlier, so different arms see the same fault realisation for the same request.
    fault_crn: Option<(u64, u64)>,
}

/// Deterministic uniform [0,1) value from (seed, key, bucket) (FNV-1a mixed through SplitMix64).
fn crn_unit(seed: u64, key: &str, bucket: u64) -> f64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ seed;
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h ^= bucket.wrapping_mul(0x9E3779B97F4A7C15);
    StreamRng::new(h).next_f64()
}

impl SimulatedOrigin {
    pub fn new(capacity: usize, distribution: ServiceDistribution) -> Self {
        Self {
            base_capacity: capacity.max(1),
            distribution,
            scenario: DegradationScenario::None,
            server_busy_until: BinaryHeap::with_capacity(capacity.max(1)),
            total_dispatched: 0,
            total_completed: 0,
            total_errors: 0,
            peak_in_flight: 0,
            sum_queue_wait_us: 0,
            sum_service_time_us: 0,
            fault_crn: None,
        }
    }

    /// Enables common-random-number brownout faults keyed by (seed, key, `bucket_us`-wide dispatch-time bucket).
    pub fn with_fault_crn(mut self, seed: u64, bucket_us: u64) -> Self {
        self.fault_crn = Some((seed, bucket_us.max(1)));
        self
    }

    pub fn with_scenario(mut self, scenario: DegradationScenario) -> Self {
        self.scenario = scenario;
        self
    }

    pub fn set_scenario(&mut self, scenario: DegradationScenario) {
        self.scenario = scenario;
    }

    fn capacity_factor(s: &DegradationScenario, now_us: u64) -> f64 {
        match s {
            DegradationScenario::CapacityStepDown { at_us, factor } if now_us >= *at_us => *factor,
            DegradationScenario::RecoveryRamp { start_us, end_us, initial_capacity_factor } => {
                if now_us < *start_us {
                    *initial_capacity_factor
                } else if now_us >= *end_us {
                    1.0
                } else {
                    let progress = (now_us - start_us) as f64 / (end_us - start_us) as f64;
                    initial_capacity_factor + (1.0 - initial_capacity_factor) * progress
                }
            }
            DegradationScenario::CapacityRecovery { start_us, end_us, initial_capacity_factor } => {
                if now_us < *start_us || now_us >= *end_us {
                    1.0
                } else {
                    let progress = (now_us - start_us) as f64 / (end_us - start_us) as f64;
                    initial_capacity_factor + (1.0 - initial_capacity_factor) * progress
                }
            }
            DegradationScenario::Composite(parts) => parts.iter().map(|p| Self::capacity_factor(p, now_us)).product(),
            _ => 1.0,
        }
    }

    /// Effective capacity at virtual timestamp `now_us`.
    pub fn effective_capacity(&self, now_us: u64) -> usize {
        let f = Self::capacity_factor(&self.scenario, now_us);
        if (f - 1.0).abs() < f64::EPSILON {
            self.base_capacity
        } else {
            ((self.base_capacity as f64 * f).round() as usize).max(1)
        }
    }

    fn latency_factor(s: &DegradationScenario, now_us: u64) -> f64 {
        match s {
            DegradationScenario::LatencyInflation { at_us, factor } if now_us >= *at_us => *factor,
            DegradationScenario::GradualInflation { start_us, end_us, final_factor } => {
                if now_us <= *start_us {
                    1.0
                } else if now_us >= *end_us {
                    *final_factor
                } else {
                    let progress = (now_us - start_us) as f64 / (end_us - start_us) as f64;
                    1.0 + (final_factor - 1.0) * progress
                }
            }
            DegradationScenario::Composite(parts) => parts.iter().map(|p| Self::latency_factor(p, now_us)).product(),
            _ => 1.0,
        }
    }

    /// Latency inflation multiplier at virtual timestamp `now_us`.
    pub fn latency_multiplier(&self, now_us: u64) -> f64 {
        Self::latency_factor(&self.scenario, now_us)
    }

    fn failure_of(s: &DegradationScenario, now_us: u64, draw: &mut dyn FnMut() -> f64) -> Option<u16> {
        match s {
            DegradationScenario::Outage { start_us, end_us }
                if (*start_us..=*end_us).contains(&now_us) => Some(503),
            DegradationScenario::Brownout { start_us, end_us, error_rate }
                if (*start_us..=*end_us).contains(&now_us) && draw() < *error_rate => Some(500),
            DegradationScenario::Flapping { start_us, end_us, cycle_us, down_duration_us }
                if (*start_us..=*end_us).contains(&now_us) => {
                    let offset = (now_us - start_us) % cycle_us;
                    if offset < *down_duration_us {
                        Some(503)
                    } else {
                        None
                    }
                }
            DegradationScenario::Composite(parts) => {
                for p in parts {
                    if let Some(c) = Self::failure_of(p, now_us, draw) {
                        return Some(c);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Check if origin is currently in full outage or brownout.
    pub fn check_failure(&self, now_us: u64, rng: &mut StreamRng) -> Option<u16> {
        let mut draw = || rng.next_f64();
        Self::failure_of(&self.scenario, now_us, &mut draw)
    }

    fn cost_factor(s: &DegradationScenario, key: &str, body_bytes: usize) -> f64 {
        match s {
            DegradationScenario::HeterogeneousCost => {
                // Stable hash-based cost multiplier in [0.5, 3.0]
                let mut h = 5381u64;
                for b in key.bytes() {
                    h = ((h << 5).wrapping_add(h)).wrapping_add(b as u64);
                }
                let hash_mod = (h % 100) as f64 / 100.0; // [0.0, 0.99]
                let base_mult = 0.5 + 2.5 * hash_mod;
                let size_mult = 1.0 + (body_bytes as f64 / 65536.0).min(2.0);
                base_mult * size_mult
            }
            DegradationScenario::Composite(parts) => parts.iter().map(|p| Self::cost_factor(p, key, body_bytes)).product(),
            _ => 1.0,
        }
    }

    /// Heterogeneous cost factor for a key or body size.
    pub fn key_cost_multiplier(&self, key: &str, body_bytes: usize) -> f64 {
        Self::cost_factor(&self.scenario, key, body_bytes)
    }

    /// Drain servers that have completed before `now_us`.
    fn prune_completed(&mut self, now_us: u64) {
        while let Some(&Reverse(busy_until)) = self.server_busy_until.peek() {
            if busy_until <= now_us {
                self.server_busy_until.pop();
            } else {
                break;
            }
        }
    }

    /// Process an origin fetch request at virtual timestamp `now_us`.
    pub fn fetch(
        &mut self,
        now_us: u64,
        key: &str,
        body_bytes: usize,
        rng: &mut StreamRng,
    ) -> OriginResponse {
        self.total_dispatched += 1;
        self.prune_completed(now_us);

        // Check for outage/brownout failures
        let failure = match self.fault_crn {
            Some((seed, bucket_us)) => {
                let u = crn_unit(seed, key, now_us / bucket_us);
                let mut draw = move || u;
                Self::failure_of(&self.scenario, now_us, &mut draw)
            }
            None => self.check_failure(now_us, rng),
        };
        if let Some(err_status) = failure {
            self.total_errors += 1;
            return OriginResponse {
                start_service_us: now_us,
                finish_service_us: now_us + 100, // Minimal error turnaround
                queue_wait_us: 0,
                is_error: true,
                status_code: err_status,
                body_bytes: 0,
            };
        }

        let cap = self.effective_capacity(now_us);

        // Determine server start time
        let start_service_us = if self.server_busy_until.len() < cap {
            // Free server available immediately
            now_us
        } else {
            // Must wait in queue for earliest server to finish
            let earliest = self.server_busy_until.peek().unwrap().0;
            earliest.max(now_us)
        };

        let queue_wait_us = start_service_us.saturating_sub(now_us);

        // Sample base service time
        let base_service_us = self.distribution.sample(rng);
        let lat_mult = self.latency_multiplier(now_us);
        let key_mult = self.key_cost_multiplier(key, body_bytes);
        let total_service_us = ((base_service_us as f64 * lat_mult * key_mult).round() as u64).max(1);

        let finish_service_us = start_service_us + total_service_us;

        // If at or exceeding capacity, remove earliest freed server
        if self.server_busy_until.len() >= cap {
            self.server_busy_until.pop();
        }
        self.server_busy_until.push(Reverse(finish_service_us));

        // Update telemetry
        self.total_completed += 1;
        self.sum_queue_wait_us += queue_wait_us;
        self.sum_service_time_us += total_service_us;
        self.peak_in_flight = self.peak_in_flight.max(self.server_busy_until.len());

        OriginResponse {
            start_service_us,
            finish_service_us,
            queue_wait_us,
            is_error: false,
            status_code: 200,
            body_bytes,
        }
    }

    pub fn total_dispatched(&self) -> u64 {
        self.total_dispatched
    }

    pub fn total_completed(&self) -> u64 {
        self.total_completed
    }

    pub fn total_errors(&self) -> u64 {
        self.total_errors
    }

    pub fn peak_in_flight(&self) -> usize {
        self.peak_in_flight
    }

    pub fn mean_queue_wait_us(&self) -> f64 {
        if self.total_completed == 0 {
            0.0
        } else {
            self.sum_queue_wait_us as f64 / self.total_completed as f64
        }
    }

    pub fn mean_service_time_us(&self) -> f64 {
        if self.total_completed == 0 {
            0.0
        } else {
            self.sum_service_time_us as f64 / self.total_completed as f64
        }
    }
}
