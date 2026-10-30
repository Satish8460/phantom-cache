#[cfg(test)]
mod tests {
    use phantom_cache_demo::control::pie::{PieController, PieDecision};
    use phantom_cache_demo::intel::sketch::CountMinSketch;
    use phantom_cache_demo::intel::telemetry::{RawObservation, TelemetryCollector};
    use std::sync::Arc;

    #[test]
    fn test_sketch_error_bounds() {
        let mut sketch = CountMinSketch::new(4, 1000);
        let test_key = 12345u64;

        for _ in 0..500 {
            sketch.increment(test_key);
        }

        let estimation = sketch.estimate(test_key);
        assert_eq!(estimation, 500);
    }

    #[test]
    fn test_telemetry_correctness() {
        let telemetry = TelemetryCollector::new(10);
        telemetry.record(RawObservation {
            timestamp_ms: 100,
            key_hash: 1,
            latency_ms: 10,
            hit: true,
            waiters_coalesced: 2,
        });

        assert_eq!(telemetry.len(), 1);
        assert_eq!(telemetry.calculate_hit_rate(), 1.0);
        assert_eq!(telemetry.calculate_coalescing_savings(), 2);
    }

    #[test]
    fn test_pie_insufficient_evidence() {
        let telemetry = Arc::new(TelemetryCollector::new(2000));
        let pie = PieController::new(telemetry.clone());

        // Insufficient sample count (0 < 1000)
        let decision = pie.evaluate_candidate("S3-FIFO", 100, true);
        assert_eq!(decision, PieDecision::NoAction);
    }

    #[test]
    fn test_pie_constraint_violation() {
        let telemetry = Arc::new(TelemetryCollector::new(2000));
        for i in 0..1100 {
            telemetry.record(RawObservation {
                timestamp_ms: i as u64,
                key_hash: 1,
                latency_ms: 10,
                hit: true,
                waiters_coalesced: 0,
            });
        }
        let pie = PieController::new(telemetry.clone());

        // Constraints violate (constraints_pass = false)
        let decision = pie.evaluate_candidate("S3-FIFO", 100, false);
        assert_eq!(decision, PieDecision::NoAction);
    }

    #[test]
    fn test_pie_cooldown_enforcement() {
        let telemetry = Arc::new(TelemetryCollector::new(2000));
        for i in 0..1100 {
            telemetry.record(RawObservation {
                timestamp_ms: i as u64,
                key_hash: 1,
                latency_ms: 5,
                hit: true,
                waiters_coalesced: 0,
            });
        }
        let pie = PieController::new(telemetry.clone());

        let decision1 = pie.evaluate_candidate("S3-FIFO", 100, true);
        assert_eq!(decision1, PieDecision::Commit);

        // Immediate subsequent request triggers cooldown guardrail
        let decision2 = pie.evaluate_candidate("S3-FIFO", 150, true);
        assert_eq!(decision2, PieDecision::NoAction);
    }
}
