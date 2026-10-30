use std::collections::HashMap;
use std::sync::Mutex;

pub struct CacheEntry {
    pub value: Vec<u8>,
    pub created_at_ms: u64,
    pub ttl_ms: u64,
    pub visited: bool,
}

pub struct CacheShard {
    pub map: HashMap<String, CacheEntry>,
}

pub struct PartitionedCache {
    shards: Vec<Mutex<CacheShard>>,
    shard_mask: usize,
}

impl PartitionedCache {
    pub fn new(num_shards: usize) -> Self {
        assert!(
            num_shards.is_power_of_two(),
            "Number of shards must be a power of two"
        );
        let mut shards = Vec::with_capacity(num_shards);
        for _ in 0..num_shards {
            shards.push(Mutex::new(CacheShard {
                map: HashMap::new(),
            }));
        }
        Self {
            shards,
            shard_mask: num_shards - 1,
        }
    }

    fn get_shard_index(&self, key: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) & self.shard_mask
    }

    pub fn get(&self, key: &str, now_ms: u64) -> Option<Vec<u8>> {
        let idx = self.get_shard_index(key);
        let mut shard = self.shards[idx].lock().unwrap();
        if let Some(entry) = shard.map.get_mut(key) {
            if entry.created_at_ms + entry.ttl_ms > now_ms {
                entry.visited = true;
                return Some(entry.value.clone());
            }
        }
        None
    }

    pub fn insert(&self, key: &str, value: Vec<u8>, ttl_ms: u64, now_ms: u64) {
        let idx = self.get_shard_index(key);
        let mut shard = self.shards[idx].lock().unwrap();
        shard.map.insert(
            key.to_string(),
            CacheEntry {
                value,
                created_at_ms: now_ms,
                ttl_ms,
                visited: false,
            },
        );
    }

    pub fn evict_sieve(&self, now_ms: u64) -> Option<String> {
        // Implement simple Sieve sweep over shards
        for shard_mutex in &self.shards {
            let mut shard = shard_mutex.lock().unwrap();
            let mut victim_key: Option<String> = None;

            for (k, entry) in shard.map.iter_mut() {
                if entry.created_at_ms + entry.ttl_ms <= now_ms {
                    victim_key = Some(k.clone());
                    break;
                }
                if entry.visited {
                    entry.visited = false;
                } else {
                    victim_key = Some(k.clone());
                    break;
                }
            }

            if let Some(ref k) = victim_key {
                shard.map.remove(k);
                return Some(k.clone());
            }
        }
        None
    }
}
