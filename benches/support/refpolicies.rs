//! Reference Cache Replacement Policies for PhantomCache (WP-05).
//!
//! Citations:
//! - SIEVE: Ying et al., "SIEVE is Simpler than LRU: an Efficient Turn-Key Eviction Algorithm for Web Caches", NSDI 2024.
//! - S3-FIFO: Yang et al., "FIFO queues are all you need for cache eviction", SOSP 2023.
//! - LRU: Classic Least Recently Used cache replacement.
//! - TinyLFU: Einziger et al., "TinyLFU: A Highly Efficient Cache Admission Policy", ACM Transactions on Storage 2017.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

// ---------------------------------------------------------------------------
// Trait: CachePolicy
// ---------------------------------------------------------------------------

pub trait CachePolicy<K: Clone + Eq + Hash> {
    fn get(&mut self, key: &K) -> bool;
    fn put(&mut self, key: K) -> Option<K>; // Returns evicted key if any
    fn contains(&self, key: &K) -> bool;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ---------------------------------------------------------------------------
// 1. Reference LRU
// ---------------------------------------------------------------------------

pub struct ReferenceLru<K: Clone + Eq + Hash> {
    capacity: usize,
    items: VecDeque<K>,
    set: HashMap<K, ()>,
}

impl<K: Clone + Eq + Hash> ReferenceLru<K> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            capacity,
            items: VecDeque::with_capacity(capacity),
            set: HashMap::with_capacity(capacity),
        }
    }
}

impl<K: Clone + Eq + Hash> CachePolicy<K> for ReferenceLru<K> {
    fn get(&mut self, key: &K) -> bool {
        if self.set.contains_key(key) {
            // Move key to the back (most recently used)
            if let Some(idx) = self.items.iter().position(|k| k == key) {
                let k = self.items.remove(idx).unwrap();
                self.items.push_back(k);
            }
            true
        } else {
            false
        }
    }

    fn put(&mut self, key: K) -> Option<K> {
        if self.set.contains_key(&key) {
            self.get(&key);
            None
        } else {
            let mut evicted = None;
            if self.items.len() >= self.capacity {
                if let Some(old) = self.items.pop_front() {
                    self.set.remove(&old);
                    evicted = Some(old);
                }
            }
            self.items.push_back(key.clone());
            self.set.insert(key, ());
            evicted
        }
    }

    fn contains(&self, key: &K) -> bool {
        self.set.contains_key(key)
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}

// ---------------------------------------------------------------------------
// 2. Reference SIEVE (NSDI 2024)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct SieveNode<K> {
    key: K,
    visited: bool,
}

pub struct ReferenceSieve<K: Clone + Eq + Hash> {
    capacity: usize,
    queue: Vec<Option<SieveNode<K>>>,
    map: HashMap<K, usize>, // key -> index in queue
    hand: usize,
}

impl<K: Clone + Eq + Hash> ReferenceSieve<K> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            capacity,
            queue: Vec::with_capacity(capacity),
            map: HashMap::with_capacity(capacity),
            hand: 0,
        }
    }

    fn compact_if_needed(&mut self) {
        if self.queue.len() > self.capacity * 4 {
            let mut new_q = Vec::with_capacity(self.map.len());
            let mut new_map = HashMap::with_capacity(self.map.len());
            for opt in self.queue.drain(..) {
                if let Some(node) = opt {
                    let idx = new_q.len();
                    new_map.insert(node.key.clone(), idx);
                    new_q.push(Some(node));
                }
            }
            self.queue = new_q;
            self.map = new_map;
            self.hand = if !self.queue.is_empty() {
                self.queue.len() - 1
            } else {
                0
            };
        }
    }
}

impl<K: Clone + Eq + Hash> CachePolicy<K> for ReferenceSieve<K> {
    fn get(&mut self, key: &K) -> bool {
        if let Some(&idx) = self.map.get(key) {
            if let Some(ref mut node) = self.queue[idx] {
                node.visited = true;
                return true;
            }
        }
        false
    }

    fn put(&mut self, key: K) -> Option<K> {
        if self.get(&key) {
            return None;
        }

        let mut evicted = None;
        if self.map.len() >= self.capacity {
            // Evict using SIEVE sweep
            let mut inspected = 0;
            let total_valid = self.map.len();
            while inspected < total_valid * 2 + 1 {
                if self.queue.is_empty() {
                    break;
                }
                if self.hand >= self.queue.len() {
                    self.hand = self.queue.len().saturating_sub(1);
                }
                if let Some(ref mut node) = self.queue[self.hand] {
                    if node.visited {
                        node.visited = false;
                        self.hand = if self.hand == 0 {
                            self.queue.len() - 1
                        } else {
                            self.hand - 1
                        };
                    } else {
                        // Found victim
                        let victim_node = self.queue[self.hand].take().unwrap();
                        self.map.remove(&victim_node.key);
                        evicted = Some(victim_node.key);
                        self.hand = if self.hand == 0 {
                            self.queue.len() - 1
                        } else {
                            self.hand - 1
                        };
                        break;
                    }
                    inspected += 1;
                } else {
                    self.hand = if self.hand == 0 {
                        self.queue.len() - 1
                    } else {
                        self.hand - 1
                    };
                }
            }
        }

        let new_idx = self.queue.len();
        self.queue.push(Some(SieveNode {
            key: key.clone(),
            visited: false,
        }));
        self.map.insert(key, new_idx);
        self.compact_if_needed();
        evicted
    }

    fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    fn len(&self) -> usize {
        self.map.len()
    }
}

// ---------------------------------------------------------------------------
// 3. Reference S3-FIFO (SOSP 2023)
// ---------------------------------------------------------------------------

pub struct ReferenceS3Fifo<K: Clone + Eq + Hash> {
    capacity: usize,
    small_cap: usize,
    small_q: VecDeque<K>,
    main_q: VecDeque<K>,
    ghost_q: VecDeque<K>,
    ghost_set: HashMap<K, ()>,
    freq: HashMap<K, u8>,
}

impl<K: Clone + Eq + Hash> ReferenceS3Fifo<K> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity >= 2);
        let small_cap = (capacity / 10).max(1);
        Self {
            capacity,
            small_cap,
            small_q: VecDeque::new(),
            main_q: VecDeque::new(),
            ghost_q: VecDeque::new(),
            ghost_set: HashMap::new(),
            freq: HashMap::new(),
        }
    }
}

impl<K: Clone + Eq + Hash> CachePolicy<K> for ReferenceS3Fifo<K> {
    fn get(&mut self, key: &K) -> bool {
        if let Some(f) = self.freq.get_mut(key) {
            *f = (*f + 1).min(3);
            true
        } else {
            false
        }
    }

    fn put(&mut self, key: K) -> Option<K> {
        if self.get(&key) {
            return None;
        }

        let mut evicted = None;

        // Ensure total resident <= capacity before insertion
        while self.small_q.len() + self.main_q.len() >= self.capacity {
            if self.small_q.len() >= self.small_cap {
                // Evict from small queue
                if let Some(victim) = self.small_q.pop_front() {
                    let f = self.freq.remove(&victim).unwrap_or(0);
                    if f > 1 {
                        // Promote to main
                        self.main_q.push_back(victim.clone());
                        self.freq.insert(victim, 0);
                    } else {
                        // Insert into ghost
                        if self.ghost_q.len() >= self.capacity {
                            if let Some(old_g) = self.ghost_q.pop_front() {
                                self.ghost_set.remove(&old_g);
                            }
                        }
                        self.ghost_q.push_back(victim.clone());
                        self.ghost_set.insert(victim.clone(), ());
                        evicted = Some(victim);
                        break;
                    }
                }
            } else {
                // Evict from main queue
                let mut found = false;
                while let Some(candidate) = self.main_q.pop_front() {
                    let f = self.freq.get(&candidate).copied().unwrap_or(0);
                    if f > 0 {
                        self.freq.insert(candidate.clone(), f - 1);
                        self.main_q.push_back(candidate);
                    } else {
                        self.freq.remove(&candidate);
                        evicted = Some(candidate);
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }
        }

        if self.ghost_set.remove(&key).is_some() {
            // In ghost queue -> insert to main
            self.main_q.push_back(key.clone());
            self.freq.insert(key, 0);
        } else {
            // New entry -> insert to small
            self.small_q.push_back(key.clone());
            self.freq.insert(key, 0);
        }

        evicted
    }

    fn contains(&self, key: &K) -> bool {
        self.freq.contains_key(key)
    }

    fn len(&self) -> usize {
        self.small_q.len() + self.main_q.len()
    }
}

// ---------------------------------------------------------------------------
// Golden Tests & Validations
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_golden_trace() {
        let mut lru = ReferenceLru::new(3);
        assert_eq!(lru.put("A"), None);
        assert_eq!(lru.put("B"), None);
        assert_eq!(lru.put("C"), None);
        assert!(lru.get(&"A")); // A touched, order: B, C, A
        assert_eq!(lru.put("D"), Some("B")); // B evicted
        assert!(!lru.contains(&"B"));
        assert!(lru.contains(&"A"));
        assert!(lru.contains(&"C"));
        assert!(lru.contains(&"D"));
    }

    #[test]
    fn test_sieve_golden_trace() {
        let mut sieve = ReferenceSieve::new(3);
        sieve.put("A");
        sieve.put("B");
        sieve.put("C");
        sieve.get(&"B"); // B visited=true
        let evicted = sieve.put("D");
        // Hand sweeps from tail: C unvisited -> evicted
        assert_eq!(evicted, Some("C"));
        assert!(!sieve.contains(&"C"));
        assert!(sieve.contains(&"A"));
        assert!(sieve.contains(&"B"));
        assert!(sieve.contains(&"D"));
    }

    #[test]
    fn test_s3fifo_golden_trace() {
        let mut s3 = ReferenceS3Fifo::new(10);
        for i in 0..10 {
            s3.put(i);
        }
        assert_eq!(s3.len(), 10);
        let evicted = s3.put(100);
        assert!(evicted.is_some());
        assert_eq!(s3.len(), 10);
    }
}

