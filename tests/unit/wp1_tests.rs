#[cfg(test)]
mod tests {
    use phantom_cache_demo::core::cache::{
        CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache,
    };
    use phantom_cache_demo::core::coalesce::{CoalesceKey, Coalescer, FinalizeOutcome};
    use phantom_cache_demo::sim::clock::{Clock, VirtualClock};
    use phantom_cache_demo::sim::rng::{RngSource, SeededRngSource};
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    #[test]
    fn test_virtual_clock() {
        let clock = VirtualClock::new(100);
        assert_eq!(clock.now_ms(), 100);
        clock.advance_ms(50);
        assert_eq!(clock.now_ms(), 150);
    }

    #[test]
    fn test_seeded_rng_determinism() {
        let rng1 = SeededRngSource::new(42);
        let rng2 = SeededRngSource::new(42);
        assert_eq!(rng1.next_u64(), rng2.next_u64());
        assert_eq!(rng1.next_f64(), rng2.next_f64());
    }

    #[test]
    fn test_partitioned_cache_ops() {
        let cache = PartitionedCache::new(4);
        cache.insert("hotel_123", vec![1, 2, 3], 100, 0);
        assert_eq!(cache.get("hotel_123", 50), Some(vec![1, 2, 3]));
        // Test TTL expiry
        assert_eq!(cache.get("hotel_123", 150), None);
    }

    #[test]
    fn test_coalescer_single_flight() {
        let coalescer = Coalescer::new(10);
        let key = CoalesceKey {
            path: "/availability/123".to_string(),
            auth_token_hash: "secret_hash".to_string(),
            tenant_id: "tenant_A".to_string(),
        };

        // First registration returns None (initiator)
        let rx1 = coalescer.register(key.clone()).unwrap();
        assert!(rx1.is_none());

        // Second registration returns Receiver (waiter)
        let rx2 = coalescer.register(key.clone()).unwrap();
        assert!(rx2.is_some());
    }

    #[test]
    fn test_core_200k_unique_key_flood_strict_capacity() {
        let cache = Arc::new(PartitionedCache::new(4));
        let max_per_shard = 2500;
        cache.set_capacity_limits(max_per_shard, 1000);

        let total_keys = 200_000;
        for i in 0..total_keys {
            let key = CacheKey::new(None, &format!("/key_{}", i), None);
            match cache.begin_fetch_atomic(&key, 1000) {
                FetchRegistrationResult::Initiate(reg) => {
                    let res = Arc::new(CachedResource {
                        status: 200,
                        content_type: "application/json".to_string(),
                        body: Arc::new(vec![1, 2, 3]),
                        etag: None,
                        created_at_ms: 1000,
                        ttl_ms: 60_000,
                    });
                    let guard = reg.into_guard();
                    guard.complete(FinalizeOutcome::Success(res));
                }
                _ => panic!("Expected Initiate for cold unique key {}", i),
            }
        }

        // Strict capacity checks across all 4 shards
        let mut total_entries = 0;
        for (idx, shard_mutex) in cache.shards.iter().enumerate() {
            let shard = shard_mutex.lock().unwrap();
            assert!(
                shard.map.len() <= max_per_shard,
                "Shard {} exceeded max_entries_per_shard: len={}, max={}",
                idx,
                shard.map.len(),
                max_per_shard
            );
            total_entries += shard.map.len();
        }

        assert!(
            total_entries <= 4 * max_per_shard,
            "Total cache entries exceeded strict capacity: {}",
            total_entries
        );

        let evictions = cache.counters.evictions_total.load(Ordering::Relaxed);
        assert_eq!(
            evictions,
            (total_keys - total_entries) as u64,
            "Evictions must exactly match total_keys minus final entries"
        );
        assert_eq!(
            cache.counters.cache_misses.load(Ordering::Relaxed),
            total_keys as u64
        );
    }
}
