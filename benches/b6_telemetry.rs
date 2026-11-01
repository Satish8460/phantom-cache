//! Benchmark B6: Telemetry Overhead (Core C4)
//!
//! Specifications:
//! - C4: `record()` at 8 threads and `calculate_percentiles()` at production buffer size 10,000.
//! - Non-core: `record()` at 1 thread / 1,000 buffer size.

use phantom_cache_demo::intel::telemetry::{ObservationKind, RawObservation, TelemetryCollector};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

fn run_telemetry_record_bench(num_threads: usize, duration_secs: u64, buffer_size: usize) -> serde_json::Value {
    let collector = Arc::new(TelemetryCollector::new(buffer_size));
    let running = Arc::new(AtomicBool::new(true));
    let total_records = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::new();

    for t_id in 0..num_threads {
        let coll = collector.clone();
        let r = running.clone();
        let counter = total_records.clone();

        handles.push(thread::spawn(move || {
            let mut local: u64 = 0;
            let now = 1_000_000;
            while r.load(Ordering::Relaxed) {
                coll.record(RawObservation::new(
                    now,
                    (t_id as u64 * 1000) + local, // key_hash
                    (local % 5000) as u32,       // latency_ms
                    local.is_multiple_of(2),     // hit
                    1,                           // waiters_coalesced
                    ObservationKind::Get,
                ));
                local += 1;
            }
            counter.fetch_add(local, Ordering::SeqCst);
        }));
    }

    let start = Instant::now();
    thread::sleep(Duration::from_secs(duration_secs));
    running.store(false, Ordering::SeqCst);

    for h in handles {
        h.join().unwrap();
    }

    let elapsed = start.elapsed().as_secs_f64();
    let records = total_records.load(Ordering::SeqCst);
    let throughput = (records as f64) / elapsed;
    let avg_ns_per_record = (elapsed * 1e9) / (records as f64);

    serde_json::json!({
        "threads": num_threads,
        "buffer_size": buffer_size,
        "records": records,
        "throughput_records_sec": throughput,
        "avg_ns_per_record": avg_ns_per_record
    })
}

fn run_percentiles_bench(buffer_size: usize, iterations: usize) -> serde_json::Value {
    let collector = Arc::new(TelemetryCollector::new(buffer_size));
    let now = 1_000_000;
    // Fill buffer
    for i in 0..buffer_size {
        collector.record(RawObservation::new(
            now,
            i as u64,                   // key_hash
            ((i * 17) % 10_000) as u32, // latency_ms
            i % 2 == 0,                 // hit
            1,                          // waiters_coalesced
            ObservationKind::Get,
        ));
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let p = collector.calculate_percentiles();
        black_box(p);
    }
    let elapsed_ns = (start.elapsed().as_nanos() as f64) / (iterations as f64);

    serde_json::json!({
        "buffer_size": buffer_size,
        "iterations": iterations,
        "calculate_percentiles_avg_ns": elapsed_ns
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let duration = if args.len() > 1 && args[1] == "--quick" {
        2
    } else {
        4
    };

    // Core C4: 8 threads, 10,000 buffer size
    let c4_record = run_telemetry_record_bench(8, duration, 10_000);
    let c4_percentiles = run_percentiles_bench(10_000, 100);

    // Non-core: 1 thread, 1,000 buffer size
    let noncore_record = run_telemetry_record_bench(1, duration, 1_000);
    let noncore_percentiles = run_percentiles_bench(1_000, 500);

    let output = serde_json::json!({
        "benchmark": "B6",
        "c4_record_8_threads": c4_record,
        "c4_percentiles_10k_buffer": c4_percentiles,
        "noncore_record_1_thread": noncore_record,
        "noncore_percentiles_1k_buffer": noncore_percentiles
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
