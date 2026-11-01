use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::core::coalesce::FinalizeOutcome;
use phantom_cache_demo::intel::diagnostics::{CacheCounters, DiagnosticBuffer};
use proptest::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

fn make_key(tenant: &str, path: &str) -> CacheKey {
    CacheKey::new(Some(tenant), path, None)
}

fn make_resource(body: &[u8], ttl_ms: u64, created_at_ms: u64) -> CachedResource {
    CachedResource {
        status: 200,
        content_type: "application/json".to_string(),
        body: Arc::new(body.to_vec()),
        etag: Some("\"etag-test\"".to_string()),
        created_at_ms,
        ttl_ms,
    }
}

// ============================================================================
// 1. PROTOCOL MODEL FOR EXHAUSTIVE INTERLEAVING VERIFICATION
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelAction {
    BeginFetch(u8),     // key index (0 or 1)
    CompleteFetch(u8),  // key index (0 or 1)
    AbortFetch(u8),     // key index (0 or 1)
    DeleteKey(u8),      // key index (0 or 1)
    InvalidateTenant,   // invalidate tenant 0
    EvictVictim,        // select and evict
}

#[derive(Debug, Clone)]
struct ModelKeyEntry {
    _payload: u64,
    size: usize,
    generation: u64,
    tenant: u8,
}

#[derive(Debug, Clone)]
struct ModelCacheState {
    resident: HashMap<u8, ModelKeyEntry>,
    in_flight: HashMap<u8, (u64, u32)>, // key -> (generation, waiters)
    key_generations: HashMap<u8, u64>,
    tenant_epoch: u64,
    resident_bytes: usize,
    skip_generation_check_mutation: bool,
}

impl ModelCacheState {
    fn new(skip_generation_check_mutation: bool) -> Self {
        Self {
            resident: HashMap::new(),
            in_flight: HashMap::new(),
            key_generations: HashMap::new(),
            tenant_epoch: 0,
            resident_bytes: 0,
            skip_generation_check_mutation,
        }
    }

    fn step(&mut self, action: ModelAction) -> Result<(), &'static str> {
        match action {
            ModelAction::BeginFetch(k) => {
                let gen = self.key_generations.entry(k).or_insert(0);
                if let Some((_, waiters)) = self.in_flight.get_mut(&k) {
                    // Coalesce into existing leader
                    *waiters += 1;
                } else {
                    // Become leader
                    self.in_flight.insert(k, (*gen, 0));
                }
            }
            ModelAction::CompleteFetch(k) => {
                if let Some((inflight_gen, _waiters)) = self.in_flight.remove(&k) {
                    let current_gen = *self.key_generations.get(&k).unwrap_or(&0);
                    // Fencing check: if generation matches, store
                    let accept = if self.skip_generation_check_mutation {
                        true // BUG INJECTED
                    } else {
                        inflight_gen == current_gen
                    };

                    if accept {
                        let size = 100;
                        if let Some(old) = self.resident.insert(
                            k,
                            ModelKeyEntry {
                                _payload: 42,
                                size,
                                generation: inflight_gen,
                                tenant: 0,
                            },
                        ) {
                            self.resident_bytes -= old.size;
                        }
                        self.resident_bytes += size;
                    }
                }
            }
            ModelAction::AbortFetch(k) => {
                self.in_flight.remove(&k);
            }
            ModelAction::DeleteKey(k) => {
                // Increment generation fence
                let gen = self.key_generations.entry(k).or_insert(0);
                *gen += 1;

                if let Some(old) = self.resident.remove(&k) {
                    self.resident_bytes = self.resident_bytes.saturating_sub(old.size);
                }
            }
            ModelAction::InvalidateTenant => {
                self.tenant_epoch += 1;
                // Invalidate all resident entries for tenant 0
                let mut removed_bytes = 0;
                self.resident.retain(|_, entry| {
                    if entry.tenant == 0 {
                        removed_bytes += entry.size;
                        false
                    } else {
                        true
                    }
                });
                self.resident_bytes = self.resident_bytes.saturating_sub(removed_bytes);
            }
            ModelAction::EvictVictim => {
                // Invariant C4: Must not evict pinned/in-flight keys
                let evictable: Vec<u8> = self
                    .resident
                    .keys()
                    .copied()
                    .filter(|k| !self.in_flight.contains_key(k))
                    .collect();

                if let Some(&victim) = evictable.first() {
                    let entry = self.resident.remove(&victim).unwrap();
                    self.resident_bytes = self.resident_bytes.saturating_sub(entry.size);
                }
            }
        }

        self.assert_invariants()
    }

    fn assert_invariants(&self) -> Result<(), &'static str> {
        // Invariant C1: No stale fill stored after delete
        for (k, entry) in &self.resident {
            let current_gen = *self.key_generations.get(k).unwrap_or(&0);
            if entry.generation < current_gen {
                return Err("Invariant C1 violated: stale fill stored after delete");
            }
        }

        // Invariant C5: Logical bytes accounting equality
        let actual_bytes: usize = self.resident.values().map(|e| e.size).sum();
        if self.resident_bytes != actual_bytes {
            return Err("Invariant C5 violated: resident bytes accounting mismatch");
        }

        Ok(())
    }
}

fn generate_action_sequences(depth: usize, current: &mut Vec<ModelAction>, out: &mut Vec<Vec<ModelAction>>) {
    if current.len() == depth {
        out.push(current.clone());
        return;
    }

    let possible = [
        ModelAction::BeginFetch(0),
        ModelAction::BeginFetch(1),
        ModelAction::CompleteFetch(0),
        ModelAction::CompleteFetch(1),
        ModelAction::AbortFetch(0),
        ModelAction::DeleteKey(0),
        ModelAction::DeleteKey(1),
        ModelAction::InvalidateTenant,
        ModelAction::EvictVictim,
    ];

    for action in possible {
        current.push(action);
        generate_action_sequences(depth, current, out);
        current.pop();
    }
}

#[test]
fn test_exhaustive_protocol_model_invariants() {
    let mut sequences = Vec::new();
    let mut current = Vec::new();
    // 5-step exhaustive permutation across 9 action variants: 9^5 = 59,049 interleavings
    generate_action_sequences(5, &mut current, &mut sequences);

    println!("Verifying {} exhaustive 5-step interleaving sequences...", sequences.len());
    let mut verified = 0;

    for seq in &sequences {
        let mut state = ModelCacheState::new(false);
        for &action in seq {
            if let Err(err) = state.step(action) {
                panic!("Sequence {:?} failed with invariant violation: {}", seq, err);
            }
        }
        verified += 1;
    }

    assert_eq!(verified, sequences.len());
    println!("Exhaustive protocol model verification PASS across {} interleavings", verified);
}

#[test]
fn test_protocol_model_mutation_detector() {
    // Deliberately skip generation check in model; must detect violation on Delete then Complete
    let seq = vec![
        ModelAction::BeginFetch(0),
        ModelAction::DeleteKey(0),
        ModelAction::CompleteFetch(0),
    ];

    let mut state = ModelCacheState::new(true); // Mutation enabled!
    let mut detected = false;
    for action in seq {
        if state.step(action).is_err() {
            detected = true;
            break;
        }
    }

    assert!(detected, "Mutation detector must catch skipped generation check!");
}

// ============================================================================
// 2. REAL CACHE CONCURRENT STRESS VERIFICATION
// ============================================================================

#[test]
fn test_real_cache_concurrent_stress_schedule() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters, diagnostics));
    let stop = Arc::new(AtomicBool::new(false));
    let barrier = Arc::new(Barrier::new(8));
    let ops_completed = Arc::new(AtomicU64::new(0));

    let mut handles = Vec::new();

    // Spawn 7 concurrent worker threads performing mixed cache operations
    for thread_id in 0..7 {
        let c = Arc::clone(&cache);
        let s = Arc::clone(&stop);
        let b = Arc::clone(&barrier);
        let ops = Arc::clone(&ops_completed);

        handles.push(thread::spawn(move || {
            b.wait();
            let mut lcg = (thread_id as u64 + 1).wrapping_mul(6364136223846793005);

            while !s.load(Ordering::Relaxed) {
                lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let key_idx = (lcg % 20) as u8;
                let key = make_key("t1", &format!("/key-{}", key_idx));
                let op = (lcg >> 16) % 5;

                match op {
                    0 | 1 => {
                        // Read or leader fetch
                        let now_ms = 1_000_000;
                        match c.begin_fetch_atomic(&key, now_ms) {
                            FetchRegistrationResult::Initiate(reg) => {
                                let body = vec![key_idx; 256];
                                let res = make_resource(&body, 60_000, now_ms);
                                let guard = reg.into_guard();
                                if let Some(mut batch) = guard.complete(FinalizeOutcome::Success(Arc::new(res))) {
                                    batch.deliver();
                                }
                            }
                            FetchRegistrationResult::Hit(_) => {}
                            FetchRegistrationResult::Wait(_) => {}
                            FetchRegistrationResult::Rejected(_) => {}
                        }
                    }
                    2 => {
                        // Delete with status
                        let _ = c.delete_with_status(&key);
                    }
                    3 => {
                        // Invalidate tenant
                        let _ = c.try_invalidate_tenant("t1");
                    }
                    4 => {
                        // Run lazy sweeper tick
                        let _ = c.sweep_lazy_reclamation(16, 1_000_000);
                    }
                    _ => unreachable!(),
                }

                ops.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // 1 sweeper thread continuously running ticks
    {
        let c = Arc::clone(&cache);
        let s = Arc::clone(&stop);
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            b.wait();
            let mut tick_now = 1_000_000u64;
            while !s.load(Ordering::Relaxed) {
                c.sweep_lazy_reclamation(32, tick_now);
                tick_now += 100;
                thread::yield_now();
            }
        }));
    }

    // Run stress for 350 ms
    thread::sleep(Duration::from_millis(350));
    stop.store(true, Ordering::Relaxed);

    for h in handles {
        h.join().unwrap();
    }

    let total_ops = ops_completed.load(Ordering::Relaxed);
    println!("Completed {} concurrent operations across 8 threads", total_ops);
    assert!(total_ops > 1000);

    // Assert strict accounting invariant on real cache post-run: accounted == recorded
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(
        accounted, recorded,
        "Audit bytes must succeed post-stress (accounted={}, recorded={})",
        accounted, recorded
    );
}

#[test]
fn test_real_cache_mutation_generation_fence() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(1, counters.clone(), diagnostics));
    let key = make_key("t1", "/mutation_fence_key");

    // 1. Begin fetch as leader
    let reg = match cache.begin_fetch_atomic(&key, 1000) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard = reg.into_guard();

    // 2. Interleaving: DELETE occurs while origin fetch is in flight!
    cache.delete_with_status(&key);

    let stale_before = counters.stale_writes_prevented.load(Ordering::Relaxed);

    // 3. Origin returns with data and tries to finalize
    let res = make_resource(b"stale_body", 60_000, 1000);
    if let Some(mut batch) = guard.complete(FinalizeOutcome::Success(Arc::new(res))) {
        batch.deliver();
    }

    // Invariant: generation fencing caught the mismatch!
    let stale_after = counters.stale_writes_prevented.load(Ordering::Relaxed);
    assert_eq!(stale_after, stale_before + 1);

    // Verify key was NOT resurrected into cache
    let entry = cache.get_resource(&key, 1001);
    assert!(entry.is_none(), "Key must not be resurrected after delete!");

    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, 0);
    assert_eq!(recorded, 0);
}

// ============================================================================
// 3. PROPTEST STATE-MACHINE TESTS
// ============================================================================

#[derive(Debug, Clone)]
enum CacheOp {
    Insert { key_idx: usize, val_len: usize },
    Get { key_idx: usize },
    Delete { key_idx: usize },
    InvalidateTenant,
}

prop_compose! {
    fn arb_cache_op()(
        op_type in 0..4usize,
        key_idx in 0..10usize,
        val_len in 10..500usize,
    ) -> CacheOp {
        match op_type {
            0 | 1 => CacheOp::Insert { key_idx, val_len },
            2 => CacheOp::Get { key_idx },
            3 => CacheOp::Delete { key_idx },
            _ => CacheOp::InvalidateTenant,
        }
    }
}

proptest! {
    #[test]
    fn test_proptest_cache_invariants(ops in prop::collection::vec(arb_cache_op(), 1..60)) {
        let counters = Arc::new(CacheCounters::new());
        let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
        let cache = Arc::new(PartitionedCache::new_with_diagnostics(2, counters, diagnostics));
        let mut shadow: HashMap<String, usize> = HashMap::new();
        let now_ms = 1_000_000;

        for op in ops {
            match op {
                CacheOp::Insert { key_idx, val_len } => {
                    let path = format!("/prop-k{}", key_idx);
                    let key = make_key("t_prop", &path);
                    if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, now_ms) {
                        let res = make_resource(&vec![7u8; val_len], 60_000, now_ms);
                        let guard = reg.into_guard();
                        if let Some(mut batch) = guard.complete(FinalizeOutcome::Success(Arc::new(res))) {
                            batch.deliver();
                            shadow.insert(path, val_len);
                        }
                    }
                }
                CacheOp::Get { key_idx } => {
                    let path = format!("/prop-k{}", key_idx);
                    let key = make_key("t_prop", &path);
                    let res = cache.get_resource(&key, now_ms);
                    if let Some(entry) = res {
                        prop_assert_eq!(entry.status, 200);
                    }
                }
                CacheOp::Delete { key_idx } => {
                    let path = format!("/prop-k{}", key_idx);
                    let key = make_key("t_prop", &path);
                    let _ = cache.delete_with_status(&key);
                    shadow.remove(&path);
                    // Post-condition: deleted key is NEVER returned immediately
                    prop_assert!(cache.get_resource(&key, now_ms).is_none());
                }
                CacheOp::InvalidateTenant => {
                    let _ = cache.try_invalidate_tenant("t_prop");
                    // Post-condition: all tenant keys are immediately unobservable
                    for path in shadow.keys() {
                        let key = make_key("t_prop", path);
                        prop_assert!(cache.get_resource(&key, now_ms).is_none());
                    }
                    shadow.clear();
                }
            }

            // Invariant C5: logical byte audit must strictly pass after every op
            let (accounted, recorded) = cache.audit_bytes();
            prop_assert_eq!(accounted, recorded);
        }
    }
}
