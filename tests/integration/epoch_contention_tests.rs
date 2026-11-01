use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use phantom_cache_demo::core::cache::{PartitionedCache, MAX_TENANT_EPOCHS};

#[test]
fn test_concurrent_epoch_reads_scaling() {
    let cache = Arc::new(PartitionedCache::new(4));

    // Populate 100 distinct tenant epochs
    for i in 0..100 {
        cache.invalidate_tenant(&format!("tenant_{}", i));
    }

    let stop = Arc::new(AtomicBool::new(false));
    let total_lookups = Arc::new(AtomicU64::new(0));

    // Launch 8 concurrent reader threads
    let mut handles = Vec::new();
    for thread_idx in 0..8 {
        let cache = cache.clone();
        let stop = stop.clone();
        let total_lookups = total_lookups.clone();

        handles.push(thread::spawn(move || {
            let mut count = 0u64;
            while !stop.load(Ordering::Relaxed) {
                let tenant_id = format!("tenant_{}", (thread_idx * 13 + count as usize) % 100);
                let (tenant_epoch, global_epoch) = cache.get_effective_epochs(&tenant_id);
                assert_eq!(tenant_epoch, 1);
                assert_eq!(global_epoch, 0);
                count += 1;
            }
            total_lookups.fetch_add(count, Ordering::Relaxed);
        }));
    }

    // Run for 200 ms
    thread::sleep(Duration::from_millis(200));
    stop.store(true, Ordering::Relaxed);

    for h in handles {
        h.join().unwrap();
    }

    let lookups = total_lookups.load(Ordering::Relaxed);
    assert!(
        lookups > 500_000,
        "8 concurrent reader threads should achieve >500k lookups in 200ms (achieved {})",
        lookups
    );
}

#[test]
fn test_invalidate_during_concurrent_reads_linearizable() {
    let cache = Arc::new(PartitionedCache::new(4));
    let stop = Arc::new(AtomicBool::new(false));
    let observed_new_epoch = Arc::new(AtomicBool::new(false));

    // Launch reader threads
    let mut handles = Vec::new();
    for _ in 0..4 {
        let cache = cache.clone();
        let stop = stop.clone();
        let observed_new = observed_new_epoch.clone();

        handles.push(thread::spawn(move || {
            let mut prev_epoch = 0u64;
            while !stop.load(Ordering::Relaxed) {
                let (epoch, _) = cache.get_effective_epochs("racing_tenant");
                // Monotonicity: epoch must never decrease
                assert!(epoch >= prev_epoch, "Tenant epoch must be monotonically non-decreasing");
                if epoch > 0 {
                    observed_new.store(true, Ordering::Relaxed);
                }
                prev_epoch = epoch;
            }
        }));
    }

    // Give readers time to spin on epoch 0
    thread::sleep(Duration::from_millis(50));

    // Writer invalidates tenant
    let new_epoch = cache.invalidate_tenant("racing_tenant");
    assert_eq!(new_epoch, 1);

    // Give readers time to observe epoch 1
    thread::sleep(Duration::from_millis(50));
    stop.store(true, Ordering::Relaxed);

    for h in handles {
        h.join().unwrap();
    }

    assert!(
        observed_new_epoch.load(Ordering::Relaxed),
        "Readers must observe updated epoch after invalidation"
    );

    let (final_epoch, _) = cache.get_effective_epochs("racing_tenant");
    assert_eq!(final_epoch, 1);
}

#[test]
fn test_invalidate_all_clears_snapshot() {
    let cache = Arc::new(PartitionedCache::new(4));

    // Invalidate 50 tenants
    for i in 0..50 {
        cache.invalidate_tenant(&format!("tenant_{}", i));
    }
    assert_eq!(cache.get_effective_epochs("tenant_5").0, 1);
    assert_eq!(cache.get_effective_epochs("tenant_5").1, 0);

    // Invalidate all
    let global_epoch = cache.invalidate_all();
    assert_eq!(global_epoch, 1);

    // Verify snapshot reflects cleared tenant epochs and updated global epoch
    let (t_epoch, g_epoch) = cache.get_effective_epochs("tenant_5");
    assert_eq!(t_epoch, 0, "Tenant epoch must reset to 0 in snapshot after invalidate_all");
    assert_eq!(g_epoch, 1, "Global epoch must be 1");
}

#[test]
fn test_capacity_exhaustion_snapshot_consistency() {
    let cache = Arc::new(PartitionedCache::new(4));

    // Invalidate MAX_TENANT_EPOCHS tenants
    for i in 0..MAX_TENANT_EPOCHS {
        cache.invalidate_tenant(&format!("cap_tenant_{}", i));
    }

    // Exceed capacity -> triggers global epoch fallback
    let overflow_tenant = format!("cap_tenant_{}", MAX_TENANT_EPOCHS);
    let next_epoch = cache.invalidate_tenant(&overflow_tenant);
    assert_eq!(next_epoch, 1);

    // Global epoch is now 1
    let (t_overflow, g_overflow) = cache.get_effective_epochs(&overflow_tenant);
    assert_eq!(t_overflow, 1);
    assert_eq!(g_overflow, 1);

    // Prior tenant was reset to 0 in snapshot, but fenced by global epoch 1
    let (t_prior, g_prior) = cache.get_effective_epochs("cap_tenant_0");
    assert_eq!(t_prior, 0);
    assert_eq!(g_prior, 1);
}
