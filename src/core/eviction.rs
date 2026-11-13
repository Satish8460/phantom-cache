//! Per-shard eviction policies and faithful literature implementations (M2-06 / WP-08).
//!
//! Citations:
//! - SIEVE: Ying et al., "SIEVE is Simpler than LRU: an Efficient Turn-Key Eviction Algorithm for Web Caches", NSDI 2024.
//! - S3-FIFO: Yang et al., "FIFO queues are all you need for cache eviction", SOSP 2023.
//! - LRU: Classic Least Recently Used cache replacement.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::core::cache::{CacheKey, EvictionPolicy};

// ---------------------------------------------------------------------------
// Trait: ShardEvictionPolicy
// ---------------------------------------------------------------------------

pub trait ShardEvictionPolicy: Send + Sync {
    /// Notification when a new key is finalized and inserted into the cache shard.
    fn on_insert(&mut self, key: &CacheKey);

    /// Notification when an existing key in the cache shard is accessed (cache hit).
    fn on_hit(&mut self, key: &CacheKey);

    /// Notification when an existing key is removed (e.g. DELETE, expired, invalidation).
    fn on_remove(&mut self, key: &CacheKey);

    /// Selects an unpinned victim key for eviction in O(1) amortized time.
    ///
    /// Invariant I5: Must NEVER select an entry where `is_pinned(&key)` returns true.
    fn select_victim(&mut self, is_pinned: &mut dyn FnMut(&CacheKey) -> bool) -> Option<CacheKey>;

    /// Rebuilds tracking structures from an existing collection of keys during policy switching.
    fn rebuild_from(&mut self, entries: &[CacheKey]);

    /// Returns the canonical name of the active eviction policy.
    fn name(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// 1. Faithful SIEVE (NSDI 2024)
// ---------------------------------------------------------------------------

struct SieveNode {
    key: CacheKey,
    visited: bool,
    prev: usize,
    next: usize,
}

pub struct FaithfulSieve {
    nodes: Vec<Option<SieveNode>>,
    free_list: Vec<usize>,
    map: HashMap<CacheKey, usize>,
    hand: usize,
}

impl FaithfulSieve {
    pub fn new(capacity_hint: usize) -> Self {
        let cap = capacity_hint.max(16);
        let mut nodes = Vec::with_capacity(cap + 1);
        // Sentinel node at index 0: prev points to tail, next points to head
        nodes.push(Some(SieveNode {
            key: CacheKey::new(None, "__sentinel__", None),
            visited: false,
            prev: 0,
            next: 0,
        }));
        Self {
            nodes,
            free_list: Vec::new(),
            map: HashMap::with_capacity(cap),
            hand: 0,
        }
    }

    fn alloc_node(&mut self, key: CacheKey) -> usize {
        if let Some(free_idx) = self.free_list.pop() {
            self.nodes[free_idx] = Some(SieveNode {
                key,
                visited: false,
                prev: 0,
                next: 0,
            });
            free_idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(Some(SieveNode {
                key,
                visited: false,
                prev: 0,
                next: 0,
            }));
            idx
        }
    }

    fn free_node(&mut self, idx: usize) {
        if idx != 0 && idx < self.nodes.len() {
            self.nodes[idx] = None;
            self.free_list.push(idx);
        }
    }

    fn remove_by_index(&mut self, idx: usize) {
        if idx == 0 || idx >= self.nodes.len() || self.nodes[idx].is_none() {
            return;
        }

        let p = self.nodes[idx].as_ref().unwrap().prev;
        let n = self.nodes[idx].as_ref().unwrap().next;

        if let Some(ref mut p_node) = self.nodes[p] {
            p_node.next = n;
        }
        if let Some(ref mut n_node) = self.nodes[n] {
            n_node.prev = p;
        }

        // If hand pointed to this node, step it backwards
        if self.hand == idx {
            self.hand = if p == 0 {
                self.nodes[0].as_ref().map(|s| s.prev).unwrap_or(0)
            } else {
                p
            };
        }

        self.free_node(idx);
    }
}

impl ShardEvictionPolicy for FaithfulSieve {
    fn on_insert(&mut self, key: &CacheKey) {
        if let Some(&idx) = self.map.get(key) {
            if let Some(ref mut node) = self.nodes[idx] {
                node.visited = true;
            }
            return;
        }

        // Allocate and insert at head (after sentinel 0)
        let idx = self.alloc_node(key.clone());
        let old_head = self.nodes[0].as_ref().unwrap().next;

        if let Some(ref mut node) = self.nodes[idx] {
            node.prev = 0;
            node.next = old_head;
        }
        if let Some(ref mut head_node) = self.nodes[old_head] {
            head_node.prev = idx;
        }
        if let Some(ref mut sentinel) = self.nodes[0] {
            sentinel.next = idx;
        }

        self.map.insert(key.clone(), idx);

        // If hand was uninitialized, point to tail
        if self.hand == 0 {
            self.hand = self.nodes[0].as_ref().map(|s| s.prev).unwrap_or(0);
        }
    }

    fn on_hit(&mut self, key: &CacheKey) {
        if let Some(&idx) = self.map.get(key) {
            if let Some(ref mut node) = self.nodes[idx] {
                node.visited = true;
            }
        }
    }

    fn on_remove(&mut self, key: &CacheKey) {
        if let Some(idx) = self.map.remove(key) {
            self.remove_by_index(idx);
        }
    }

    fn select_victim(&mut self, is_pinned: &mut dyn FnMut(&CacheKey) -> bool) -> Option<CacheKey> {
        if self.map.is_empty() {
            return None;
        }

        if self.hand == 0 || self.nodes[self.hand].is_none() {
            self.hand = self.nodes[0].as_ref().map(|s| s.prev).unwrap_or(0);
        }

        let mut inspected = 0;
        let max_inspections = self.map.len() * 2 + 2;

        while self.hand != 0 && inspected < max_inspections {
            let (is_vis, key, prev_idx) = {
                let node = self.nodes[self.hand].as_ref()?;
                (node.visited, node.key.clone(), node.prev)
            };

            let next_hand = if prev_idx == 0 {
                self.nodes[0].as_ref().map(|s| s.prev).unwrap_or(0)
            } else {
                prev_idx
            };

            if is_pinned(&key) {
                // Invariant I5: Never evict pinned state; step past
                self.hand = next_hand;
            } else if is_vis {
                if let Some(ref mut n) = self.nodes[self.hand] {
                    n.visited = false;
                }
                self.hand = next_hand;
            } else {
                // Found victim
                let victim_idx = self.hand;
                self.hand = next_hand;
                self.map.remove(&key);
                self.remove_by_index(victim_idx);
                return Some(key);
            }

            inspected += 1;
        }

        None
    }

    fn rebuild_from(&mut self, entries: &[CacheKey]) {
        self.nodes.clear();
        self.free_list.clear();
        self.map.clear();
        self.hand = 0;

        self.nodes.push(Some(SieveNode {
            key: CacheKey::new(None, "__sentinel__", None),
            visited: false,
            prev: 0,
            next: 0,
        }));

        for key in entries {
            self.on_insert(key);
        }
    }

    fn name(&self) -> &'static str {
        "sieve"
    }
}

// ---------------------------------------------------------------------------
// 2. Faithful LRU
// ---------------------------------------------------------------------------

struct LruNode {
    key: CacheKey,
    prev: usize,
    next: usize,
}

pub struct FaithfulLru {
    nodes: Vec<Option<LruNode>>,
    free_list: Vec<usize>,
    map: HashMap<CacheKey, usize>,
}

impl FaithfulLru {
    pub fn new(capacity_hint: usize) -> Self {
        let cap = capacity_hint.max(16);
        let mut nodes = Vec::with_capacity(cap + 1);
        nodes.push(Some(LruNode {
            key: CacheKey::new(None, "__sentinel__", None),
            prev: 0,
            next: 0,
        }));
        Self {
            nodes,
            free_list: Vec::new(),
            map: HashMap::with_capacity(cap),
        }
    }

    fn alloc_node(&mut self, key: CacheKey) -> usize {
        if let Some(free_idx) = self.free_list.pop() {
            self.nodes[free_idx] = Some(LruNode {
                key,
                prev: 0,
                next: 0,
            });
            free_idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(Some(LruNode {
                key,
                prev: 0,
                next: 0,
            }));
            idx
        }
    }

    fn free_node(&mut self, idx: usize) {
        if idx != 0 && idx < self.nodes.len() {
            self.nodes[idx] = None;
            self.free_list.push(idx);
        }
    }

    fn move_to_head(&mut self, idx: usize) {
        if idx == 0 || idx >= self.nodes.len() || self.nodes[idx].is_none() {
            return;
        }

        // Unlink
        let p = self.nodes[idx].as_ref().unwrap().prev;
        let n = self.nodes[idx].as_ref().unwrap().next;
        if let Some(ref mut p_node) = self.nodes[p] {
            p_node.next = n;
        }
        if let Some(ref mut n_node) = self.nodes[n] {
            n_node.prev = p;
        }

        // Link at head
        let old_head = self.nodes[0].as_ref().unwrap().next;
        if let Some(ref mut node) = self.nodes[idx] {
            node.prev = 0;
            node.next = old_head;
        }
        if let Some(ref mut head_node) = self.nodes[old_head] {
            head_node.prev = idx;
        }
        if let Some(ref mut sentinel) = self.nodes[0] {
            sentinel.next = idx;
        }
    }

    fn remove_by_index(&mut self, idx: usize) {
        if idx == 0 || idx >= self.nodes.len() || self.nodes[idx].is_none() {
            return;
        }

        let p = self.nodes[idx].as_ref().unwrap().prev;
        let n = self.nodes[idx].as_ref().unwrap().next;

        if let Some(ref mut p_node) = self.nodes[p] {
            p_node.next = n;
        }
        if let Some(ref mut n_node) = self.nodes[n] {
            n_node.prev = p;
        }

        self.free_node(idx);
    }
}

impl ShardEvictionPolicy for FaithfulLru {
    fn on_insert(&mut self, key: &CacheKey) {
        if let Some(&idx) = self.map.get(key) {
            self.move_to_head(idx);
            return;
        }

        let idx = self.alloc_node(key.clone());
        let old_head = self.nodes[0].as_ref().unwrap().next;

        if let Some(ref mut node) = self.nodes[idx] {
            node.prev = 0;
            node.next = old_head;
        }
        if let Some(ref mut head_node) = self.nodes[old_head] {
            head_node.prev = idx;
        }
        if let Some(ref mut sentinel) = self.nodes[0] {
            sentinel.next = idx;
        }

        self.map.insert(key.clone(), idx);
    }

    fn on_hit(&mut self, key: &CacheKey) {
        if let Some(&idx) = self.map.get(key) {
            self.move_to_head(idx);
        }
    }

    fn on_remove(&mut self, key: &CacheKey) {
        if let Some(idx) = self.map.remove(key) {
            self.remove_by_index(idx);
        }
    }

    fn select_victim(&mut self, is_pinned: &mut dyn FnMut(&CacheKey) -> bool) -> Option<CacheKey> {
        if self.map.is_empty() {
            return None;
        }

        // Search backward from tail (least recently used)
        let mut curr = self.nodes[0].as_ref().unwrap().prev;
        while curr != 0 {
            let key = self.nodes[curr].as_ref().unwrap().key.clone();
            if !is_pinned(&key) {
                self.map.remove(&key);
                self.remove_by_index(curr);
                return Some(key);
            }
            curr = self.nodes[curr].as_ref().unwrap().prev;
        }

        None
    }

    fn rebuild_from(&mut self, entries: &[CacheKey]) {
        self.nodes.clear();
        self.free_list.clear();
        self.map.clear();

        self.nodes.push(Some(LruNode {
            key: CacheKey::new(None, "__sentinel__", None),
            prev: 0,
            next: 0,
        }));

        for key in entries {
            self.on_insert(key);
        }
    }

    fn name(&self) -> &'static str {
        "lru"
    }
}

// ---------------------------------------------------------------------------
// 3. Faithful S3-FIFO (SOSP 2023)
// ---------------------------------------------------------------------------

pub struct FaithfulS3Fifo {
    capacity: usize,
    small_cap: usize,
    small_q: VecDeque<CacheKey>,
    main_q: VecDeque<CacheKey>,
    ghost_q: VecDeque<CacheKey>,
    ghost_set: HashSet<CacheKey>,
    freq: HashMap<CacheKey, u8>,
}

impl FaithfulS3Fifo {
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(2);
        let small_cap = (cap / 10).max(1);
        Self {
            capacity: cap,
            small_cap,
            small_q: VecDeque::with_capacity(small_cap * 2),
            main_q: VecDeque::with_capacity(cap),
            ghost_q: VecDeque::with_capacity(cap),
            ghost_set: HashSet::with_capacity(cap),
            freq: HashMap::with_capacity(cap),
        }
    }
}

impl ShardEvictionPolicy for FaithfulS3Fifo {
    fn on_insert(&mut self, key: &CacheKey) {
        if let Some(f) = self.freq.get_mut(key) {
            *f = (*f + 1).min(3);
            return;
        }

        if self.ghost_set.remove(key) {
            // Hit in ghost queue -> promote directly to main
            self.main_q.push_back(key.clone());
            self.freq.insert(key.clone(), 0);
        } else {
            // Admitted to small queue
            self.small_q.push_back(key.clone());
            self.freq.insert(key.clone(), 0);
        }
    }

    fn on_hit(&mut self, key: &CacheKey) {
        if let Some(f) = self.freq.get_mut(key) {
            *f = (*f + 1).min(3);
        }
    }

    fn on_remove(&mut self, key: &CacheKey) {
        self.freq.remove(key);
        self.ghost_set.remove(key);
    }

    fn select_victim(&mut self, is_pinned: &mut dyn FnMut(&CacheKey) -> bool) -> Option<CacheKey> {
        let total_resident = self.freq.len();
        if total_resident == 0 {
            return None;
        }

        let mut iterations = 0;
        let max_iterations = (self.small_q.len() + self.main_q.len()) * 2 + 4;

        while iterations < max_iterations {
            iterations += 1;

            if self.small_q.len() >= self.small_cap {
                // Evict from Small queue
                if let Some(candidate) = self.small_q.pop_front() {
                    // If key was removed or deleted, discard tombstone
                    let f = match self.freq.get(&candidate) {
                        Some(&val) => val,
                        None => continue,
                    };

                    if is_pinned(&candidate) {
                        // Pinned entries rotate back
                        self.small_q.push_back(candidate);
                        continue;
                    }

                    if f > 0 {
                        // Promote to Main queue
                        self.main_q.push_back(candidate.clone());
                        self.freq.insert(candidate, 0);
                    } else {
                        // Evict from Small and insert into Ghost
                        self.freq.remove(&candidate);
                        if self.ghost_q.len() >= self.capacity {
                            if let Some(old_g) = self.ghost_q.pop_front() {
                                self.ghost_set.remove(&old_g);
                            }
                        }
                        self.ghost_q.push_back(candidate.clone());
                        self.ghost_set.insert(candidate.clone());
                        return Some(candidate);
                    }
                }
            } else {
                // Evict from Main queue
                if let Some(candidate) = self.main_q.pop_front() {
                    // If key was removed or deleted, discard tombstone
                    let f = match self.freq.get(&candidate) {
                        Some(&val) => val,
                        None => continue,
                    };

                    if is_pinned(&candidate) {
                        self.main_q.push_back(candidate);
                        continue;
                    }

                    if f > 0 {
                        // Decrement and rotate
                        self.freq.insert(candidate.clone(), f - 1);
                        self.main_q.push_back(candidate);
                    } else {
                        // Evict from Main
                        self.freq.remove(&candidate);
                        return Some(candidate);
                    }
                } else if !self.small_q.is_empty() {
                    // Small queue fallback if Main is exhausted
                    if let Some(candidate) = self.small_q.pop_front() {
                        if !self.freq.contains_key(&candidate) {
                            continue;
                        }
                        if is_pinned(&candidate) {
                            self.small_q.push_back(candidate);
                            continue;
                        }
                        self.freq.remove(&candidate);
                        return Some(candidate);
                    }
                } else {
                    break;
                }
            }
        }

        None
    }

    fn rebuild_from(&mut self, entries: &[CacheKey]) {
        self.small_q.clear();
        self.main_q.clear();
        self.ghost_q.clear();
        self.ghost_set.clear();
        self.freq.clear();

        for key in entries {
            self.on_insert(key);
        }
    }

    fn name(&self) -> &'static str {
        "s3fifo"
    }
}

// ---------------------------------------------------------------------------
// 4. ClockScan (Legacy Fallback & Deprecation Alias)
// ---------------------------------------------------------------------------

pub struct ClockScan {
    keys: Vec<CacheKey>,
    visited: HashMap<CacheKey, bool>,
}

impl ClockScan {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            visited: HashMap::new(),
        }
    }
}

impl Default for ClockScan {
    fn default() -> Self {
        Self::new()
    }
}

impl ShardEvictionPolicy for ClockScan {
    fn on_insert(&mut self, key: &CacheKey) {
        if !self.visited.contains_key(key) {
            self.keys.push(key.clone());
            self.visited.insert(key.clone(), false);
        }
    }

    fn on_hit(&mut self, key: &CacheKey) {
        if let Some(vis) = self.visited.get_mut(key) {
            *vis = true;
        }
    }

    fn on_remove(&mut self, key: &CacheKey) {
        self.visited.remove(key);
        self.keys.retain(|k| k != key);
    }

    fn select_victim(&mut self, is_pinned: &mut dyn FnMut(&CacheKey) -> bool) -> Option<CacheKey> {
        let mut victim_idx = None;

        for (idx, k) in self.keys.iter().enumerate() {
            if is_pinned(k) {
                continue;
            }
            if let Some(vis) = self.visited.get_mut(k) {
                if *vis {
                    *vis = false;
                } else {
                    victim_idx = Some(idx);
                    break;
                }
            }
        }

        if victim_idx.is_none() {
            for (idx, k) in self.keys.iter().enumerate() {
                if !is_pinned(k) {
                    victim_idx = Some(idx);
                    break;
                }
            }
        }

        if let Some(idx) = victim_idx {
            let key = self.keys.swap_remove(idx);
            self.visited.remove(&key);
            Some(key)
        } else {
            None
        }
    }

    fn rebuild_from(&mut self, entries: &[CacheKey]) {
        self.keys.clear();
        self.visited.clear();
        for k in entries {
            self.on_insert(k);
        }
    }

    fn name(&self) -> &'static str {
        "clock_scan"
    }
}

// ---------------------------------------------------------------------------
// Factory helper
// ---------------------------------------------------------------------------

pub fn create_shard_eviction_policy(
    policy: EvictionPolicy,
    capacity: usize,
) -> Box<dyn ShardEvictionPolicy> {
    match policy {
        EvictionPolicy::Sieve => Box::new(FaithfulSieve::new(capacity)),
        EvictionPolicy::Lru => Box::new(FaithfulLru::new(capacity)),
        EvictionPolicy::S3Fifo => Box::new(FaithfulS3Fifo::new(capacity)),
    }
}

