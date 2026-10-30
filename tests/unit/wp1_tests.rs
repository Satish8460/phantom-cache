#[cfg(test)]
mod tests {
    use phantom_cache_demo::core::cache::PartitionedCache;
    use phantom_cache_demo::core::coalesce::{CoalesceKey, Coalescer};
    use phantom_cache_demo::sim::clock::{Clock, VirtualClock};
    use phantom_cache_demo::sim::rng::{RngSource, SeededRngSource};

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
}
