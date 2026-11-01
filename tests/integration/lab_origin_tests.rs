//! Integration tests for Phantom Lab Simulated Origin & Degradation Scenarios (M4-03 / WP-15).
//!
//! Validates:
//! - M/M/1 queueing theory analytical vs empirical wait time (Pollaczek-Khinchine / Erlang formula).
//! - M/M/c queueing theory analytical vs empirical wait time (Erlang-C formula).
//! - O-ST: Capacity step-down transition.
//! - O-LAT: Latency inflation multiplier.
//! - O-BR: Brownout error injection rate.
//! - O-OUT: Full outage 503 behavior.
//! - O-REC: Recovery capacity ramp.
//! - O-FL: Flapping periodic availability oscillations.
//! - O-HC: Heterogeneous cost scaling across keys.

#[path = "../support/lab/mod.rs"]
mod lab;

use lab::origin_sim::{DegradationScenario, ServiceDistribution, SimulatedOrigin};
use lab::rng::StreamRng;

#[test]
fn test_mm1_queueing_theory_analytical_conformance() {
    let mut rng = StreamRng::new(20001);
    // M/M/1 setup:
    // Capacity c = 1
    // Mean service time 1/mu = 1000 us (mu = 0.001)
    // Mean inter-arrival time 1/lambda = 2000 us (lambda = 0.0005)
    // Utilization rho = lambda / mu = 0.50
    // Analytical W_q = rho / (mu * (1 - rho)) = 0.50 / (0.001 * 0.50) = 1000.0 us
    let mut origin = SimulatedOrigin::new(1, ServiceDistribution::Exponential { mean_us: 1000.0 });

    let n = 40_000;
    let lambda = 0.0005; // arrivals per us
    let mut current_time_us = 0u64;

    for _ in 0..n {
        let delta = (rng.next_exponential(lambda) as u64).max(1);
        current_time_us += delta;
        let _ = origin.fetch(current_time_us, "/item", 1024, &mut rng);
    }

    let empirical_wait = origin.mean_queue_wait_us();
    let empirical_service = origin.mean_service_time_us();

    // Verify mean service time within 5% of 1000us
    assert!(
        (empirical_service - 1000.0).abs() < 50.0,
        "Empirical service time {:.2}us deviated from expected 1000us",
        empirical_service
    );

    // Verify mean queue wait within 15% of analytical 1000us
    assert!(
        (empirical_wait - 1000.0).abs() < 150.0,
        "Empirical W_q {:.2}us deviated from analytical 1000.0us",
        empirical_wait
    );
}

#[test]
fn test_mmc_queueing_theory_analytical_conformance() {
    let mut rng = StreamRng::new(20002);
    // M/M/c setup:
    // Capacity c = 4
    // Mean service time 1/mu = 4000 us (mu = 0.00025)
    // Arrival rate lambda = 0.00075 -> inter-arrival mean = 1333.33 us
    // Traffic intensity a = lambda / mu = 3.0
    // Utilization rho = a / c = 3.0 / 4 = 0.75
    // Erlang-C formula:
    // sum_{k=0}^3 3^k / k! = 1 + 3 + 4.5 + 4.5 = 13.0
    // 3^4 / (4! * (1 - 0.75)) = 81 / (24 * 0.25) = 13.5
    // C(4, 3) = 13.5 / (13.0 + 13.5) = 13.5 / 26.5 = 0.50943
    // Analytical W_q = C(4, 3) / (c*mu - lambda) = 0.50943 / (0.001 - 0.00075) = 0.50943 / 0.00025 = 2037.7 us
    let mut origin = SimulatedOrigin::new(4, ServiceDistribution::Exponential { mean_us: 4000.0 });

    let n = 40_000;
    let lambda = 0.00075;
    let mut current_time_us = 0u64;

    for _ in 0..n {
        let delta = (rng.next_exponential(lambda) as u64).max(1);
        current_time_us += delta;
        let _ = origin.fetch(current_time_us, "/item", 1024, &mut rng);
    }

    let empirical_wait = origin.mean_queue_wait_us();
    let empirical_service = origin.mean_service_time_us();

    // Verify mean service time within 5% of 4000us
    assert!(
        (empirical_service - 4000.0).abs() < 200.0,
        "Empirical service time {:.2}us deviated from expected 4000us",
        empirical_service
    );

    // Verify mean queue wait within 25% of analytical 2037.7us (due to queueing autocorrelation)
    assert!(
        (empirical_wait - 2037.7).abs() < 510.0,
        "Empirical W_q {:.2}us deviated from analytical 2037.7us",
        empirical_wait
    );
}

#[test]
fn test_capacity_step_down_ost() {
    let mut rng = StreamRng::new(20003);
    // Base capacity 10, constant service time 1000us
    // At t = 10_000_000 (10s), capacity steps down to 0.2 (2 servers)
    let step_time_us = 10_000_000;
    let mut origin = SimulatedOrigin::new(10, ServiceDistribution::Constant(1000))
        .with_scenario(DegradationScenario::CapacityStepDown {
            at_us: step_time_us,
            factor: 0.20,
        });

    assert_eq!(origin.effective_capacity(5_000_000), 10);
    assert_eq!(origin.effective_capacity(15_000_000), 2);

    // Run traffic with arrival delta = 250us (4 req/ms)
    // 10 servers can handle 10 req/ms -> no queueing before step-down
    let mut now_us = 0u64;
    for _ in 0..1000 {
        now_us += 250;
        let resp = origin.fetch(now_us, "/key", 1024, &mut rng);
        assert_eq!(resp.queue_wait_us, 0);
    }

    // After step-down at 10s: 2 servers can only handle 2 req/ms
    // Arrivals at 4 req/ms will accumulate queue delay
    now_us = step_time_us;
    let mut queued_count = 0;
    for _ in 0..100 {
        now_us += 250;
        let resp = origin.fetch(now_us, "/key", 1024, &mut rng);
        if resp.queue_wait_us > 0 {
            queued_count += 1;
        }
    }
    assert!(queued_count > 90, "Requests must queue after capacity step-down");
}

#[test]
fn test_latency_inflation_olat() {
    let mut rng = StreamRng::new(20004);
    let mut origin = SimulatedOrigin::new(10, ServiceDistribution::Constant(1000))
        .with_scenario(DegradationScenario::LatencyInflation {
            at_us: 5_000_000,
            factor: 3.5,
        });

    // Before inflation
    let r1 = origin.fetch(1_000_000, "/k", 1024, &mut rng);
    assert_eq!(r1.finish_service_us - r1.start_service_us, 1000);

    // After inflation
    let r2 = origin.fetch(6_000_000, "/k", 1024, &mut rng);
    assert_eq!(r2.finish_service_us - r2.start_service_us, 3500);
}

#[test]
fn test_brownout_obr() {
    let mut rng = StreamRng::new(20005);
    let mut origin = SimulatedOrigin::new(10, ServiceDistribution::Constant(1000))
        .with_scenario(DegradationScenario::Brownout {
            start_us: 2_000_000,
            end_us: 6_000_000,
            error_rate: 0.25, // 25% errors
        });

    // Before brownout (1s) -> 0 errors
    for _ in 0..500 {
        let resp = origin.fetch(1_000_000, "/k", 1024, &mut rng);
        assert!(!resp.is_error);
        assert_eq!(resp.status_code, 200);
    }

    // During brownout (4s) -> ~25% errors
    let n = 5_000;
    let mut errors = 0;
    for _ in 0..n {
        let resp = origin.fetch(4_000_000, "/k", 1024, &mut rng);
        if resp.is_error {
            errors += 1;
            assert_eq!(resp.status_code, 500);
        }
    }
    let observed_rate = (errors as f64) / (n as f64);
    assert!(
        (observed_rate - 0.25).abs() < 0.03,
        "Brownout error rate {:.3} deviated from expected 0.25",
        observed_rate
    );

    // After brownout (7s) -> 0 errors
    for _ in 0..500 {
        let resp = origin.fetch(7_000_000, "/k", 1024, &mut rng);
        assert!(!resp.is_error);
    }
}

#[test]
fn test_outage_oout() {
    let mut rng = StreamRng::new(20006);
    let mut origin = SimulatedOrigin::new(10, ServiceDistribution::Constant(1000))
        .with_scenario(DegradationScenario::Outage {
            start_us: 2_000_000,
            end_us: 4_000_000,
        });

    // Before outage
    let r1 = origin.fetch(1_000_000, "/k", 1024, &mut rng);
    assert!(!r1.is_error);
    assert_eq!(r1.status_code, 200);

    // During outage
    let r2 = origin.fetch(3_000_000, "/k", 1024, &mut rng);
    assert!(r2.is_error);
    assert_eq!(r2.status_code, 503);

    // After outage
    let r3 = origin.fetch(5_000_000, "/k", 1024, &mut rng);
    assert!(!r3.is_error);
    assert_eq!(r3.status_code, 200);
}

#[test]
fn test_recovery_ramp_orec() {
    let origin = SimulatedOrigin::new(10, ServiceDistribution::Constant(1000))
        .with_scenario(DegradationScenario::RecoveryRamp {
            start_us: 1_000_000,
            end_us: 5_000_000,
            initial_capacity_factor: 0.20,
        });

    assert_eq!(origin.effective_capacity(500_000), 2);
    assert_eq!(origin.effective_capacity(1_000_000), 2);
    // At 3s (halfway through ramp): 0.20 + 0.80 * 0.50 = 0.60 -> 6 servers
    assert_eq!(origin.effective_capacity(3_000_000), 6);
    assert_eq!(origin.effective_capacity(5_000_000), 10);
    assert_eq!(origin.effective_capacity(6_000_000), 10);
}

#[test]
fn test_flapping_ofl() {
    let mut rng = StreamRng::new(20007);
    let mut origin = SimulatedOrigin::new(10, ServiceDistribution::Constant(1000))
        .with_scenario(DegradationScenario::Flapping {
            start_us: 1_000_000,
            end_us: 10_000_000,
            cycle_us: 2_000_000,        // 2s period
            down_duration_us: 1_000_000, // 1s down, 1s up
        });

    // Before flapping window (500ms) -> up
    assert!(!origin.fetch(500_000, "/k", 1024, &mut rng).is_error);

    // Cycle 1: [1s..2s] is down
    assert_eq!(origin.fetch(1_500_000, "/k", 1024, &mut rng).status_code, 503);
    // Cycle 1: [2s..3s] is up
    assert_eq!(origin.fetch(2_500_000, "/k", 1024, &mut rng).status_code, 200);

    // Cycle 2: [3s..4s] is down
    assert_eq!(origin.fetch(3_500_000, "/k", 1024, &mut rng).status_code, 503);
    // Cycle 2: [4s..5s] is up
    assert_eq!(origin.fetch(4_500_000, "/k", 1024, &mut rng).status_code, 200);
}

#[test]
fn test_heterogeneous_cost_ohc() {
    let mut rng = StreamRng::new(20008);
    let mut origin = SimulatedOrigin::new(10, ServiceDistribution::Constant(1000))
        .with_scenario(DegradationScenario::HeterogeneousCost);

    let r_a = origin.fetch(0, "/alpha", 1024, &mut rng);
    let r_b = origin.fetch(0, "/beta", 1024, &mut rng);
    let r_c = origin.fetch(0, "/gamma", 1024, &mut rng);

    let dur_a = r_a.finish_service_us - r_a.start_service_us;
    let dur_b = r_b.finish_service_us - r_b.start_service_us;
    let dur_c = r_c.finish_service_us - r_c.start_service_us;

    // Keys have different hash costs, so durations must not all be identical
    assert!(dur_a != dur_b || dur_b != dur_c);

    // Repeated call for same key must yield identical cost factor
    let r_a2 = origin.fetch(0, "/alpha", 1024, &mut rng);
    let dur_a2 = r_a2.finish_service_us - r_a2.start_service_us;
    assert_eq!(dur_a, dur_a2);
}
