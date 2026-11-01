use crate::core::cache::{CacheKey, CachedResource, PartitionedCache};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

// ==========================================
// Existing types preserved for backward compatibility
// ==========================================

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CoalesceKey {
    pub path: String,
    pub auth_token_hash: String,
    pub tenant_id: String,
}

pub struct Coalescer {
    in_flight: Mutex<HashMap<CoalesceKey, Vec<oneshot::Sender<Vec<u8>>>>>,
    waiter_limit: usize,
}

impl Coalescer {
    pub fn new(waiter_limit: usize) -> Self {
        Self {
            in_flight: Mutex::new(HashMap::new()),
            waiter_limit,
        }
    }

    pub fn register(
        &self,
        key: CoalesceKey,
    ) -> Result<Option<oneshot::Receiver<Vec<u8>>>, &'static str> {
        let mut guard = self.in_flight.lock().unwrap();

        if let Some(waiters) = guard.get_mut(&key) {
            if waiters.len() >= self.waiter_limit {
                return Err("Waiter limit reached");
            }
            let (tx, rx) = oneshot::channel();
            waiters.push(tx);
            Ok(Some(rx))
        } else {
            guard.insert(key, Vec::new());
            Ok(None)
        }
    }

    pub fn resolve(&self, key: &CoalesceKey, value: Vec<u8>) {
        let mut guard = self.in_flight.lock().unwrap();
        if let Some(waiters) = guard.remove(key) {
            for tx in waiters {
                let _ = tx.send(value.clone());
            }
        }
    }
}

// ==========================================
// Milestone 1 Production Coalescer & RAII Lifecycle
// ==========================================

#[derive(Clone, Debug)]
pub enum OriginError {
    UpstreamStatus(u16),
    Timeout,
    Aborted,
    NetworkError(String),
    ResponseTooLarge,
    WaiterLimitExceeded,
    CapacityExceeded,
    TenantIdTooLong,
    /// Tenant id is empty or contains a byte outside `[A-Za-z0-9._-]` (M2).
    InvalidTenantId,
    /// Request path contains ":?", which is ambiguous with the canonical path/query separator (M2).
    InvalidPath,
    UriTooLong,
    SecurityError(crate::security::upstream::OriginSecurityError),
}

impl From<crate::security::upstream::OriginSecurityError> for OriginError {
    fn from(err: crate::security::upstream::OriginSecurityError) -> Self {
        OriginError::SecurityError(err)
    }
}

pub type CoalesceResult = Result<Arc<CachedResource>, Arc<OriginError>>;

#[derive(Clone, Debug)]
pub enum FinalizeOutcome {
    Success(Arc<CachedResource>),
    Uncacheable(Arc<CachedResource>),
    Error(Arc<OriginError>),
}

pub struct SharedOp {
    pub op_id: u64,
    pub op_gen: u64,
    pub state_instance_id: u64,
    pub waiters: Mutex<Vec<oneshot::Sender<CoalesceResult>>>,
    /// Freshness metadata for the body this op is about to store. Installed atomically with the
    /// entry by `finalize_fetch` (only if the store is accepted); discarded otherwise (M1).
    pub pending_freshness: Mutex<Option<crate::core::freshness::FreshnessMetadata>>,
}

pub struct WaiterBatch {
    waiters: Vec<oneshot::Sender<CoalesceResult>>,
    outcome: CoalesceResult,
}

impl WaiterBatch {
    pub fn new(waiters: Vec<oneshot::Sender<CoalesceResult>>, outcome: CoalesceResult) -> Self {
        Self { waiters, outcome }
    }

    pub fn deliver(&mut self) {
        for tx in self.waiters.drain(..) {
            let _ = tx.send(self.outcome.clone());
        }
    }
}

impl Drop for WaiterBatch {
    fn drop(&mut self) {
        self.deliver();
    }
}

pub struct FetchRegistration {
    pub cache: Arc<PartitionedCache>,
    pub key: CacheKey,
    pub shard_idx: usize,
    pub op_id: u64,
    pub op_gen: u64,
    pub state_instance_id: u64,
    pub shared_op: Arc<SharedOp>,
    pub disarmed: bool,
}

impl FetchRegistration {
    pub fn into_guard(mut self) -> OriginFetchGuard {
        self.disarmed = true;
        OriginFetchGuard::new(
            self.cache.clone(),
            self.key.clone(),
            self.shard_idx,
            self.op_id,
            self.op_gen,
            self.state_instance_id,
            self.shared_op.clone(),
        )
    }
}

impl Drop for FetchRegistration {
    fn drop(&mut self) {
        if !self.disarmed {
            let mut batch = self.cache.finalize_fetch(
                &self.key,
                self.shard_idx,
                self.op_id,
                self.op_gen,
                self.state_instance_id,
                &self.shared_op,
                FinalizeOutcome::Error(Arc::new(OriginError::Aborted)),
            );
            if let Some(ref mut b) = batch {
                b.deliver();
            }
        }
    }
}

pub const PHASE_PENDING: u8 = 0;
pub const PHASE_FINALIZING: u8 = 1;
pub const PHASE_COMPLETED: u8 = 2;

pub struct OriginFetchGuard {
    pub cache: Arc<PartitionedCache>,
    pub key: CacheKey,
    pub shard_idx: usize,
    pub op_id: u64,
    pub op_gen: u64,
    pub state_instance_id: u64,
    pub shared_op: Arc<SharedOp>,
    pub phase: AtomicU8,
}

impl OriginFetchGuard {
    pub fn new(
        cache: Arc<PartitionedCache>,
        key: CacheKey,
        shard_idx: usize,
        op_id: u64,
        op_gen: u64,
        state_instance_id: u64,
        shared_op: Arc<SharedOp>,
    ) -> Self {
        Self {
            cache,
            key,
            shard_idx,
            op_id,
            op_gen,
            state_instance_id,
            shared_op,
            phase: AtomicU8::new(PHASE_PENDING),
        }
    }

    pub fn complete(self, outcome: FinalizeOutcome) -> Option<WaiterBatch> {
        self.finalize_internal(outcome)
    }

    pub fn finalize_internal(&self, outcome: FinalizeOutcome) -> Option<WaiterBatch> {
        if self
            .phase
            .compare_exchange(
                PHASE_PENDING,
                PHASE_FINALIZING,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err()
        {
            return None;
        }

        let batch = self.cache.finalize_fetch(
            &self.key,
            self.shard_idx,
            self.op_id,
            self.op_gen,
            self.state_instance_id,
            &self.shared_op,
            outcome,
        );

        self.phase.store(PHASE_COMPLETED, Ordering::SeqCst);
        batch
    }
}

impl Drop for OriginFetchGuard {
    fn drop(&mut self) {
        match self.phase.load(Ordering::SeqCst) {
            PHASE_PENDING => {
                if let Some(mut batch) =
                    self.finalize_internal(FinalizeOutcome::Error(Arc::new(OriginError::Aborted)))
                {
                    batch.deliver();
                }
            }
            PHASE_FINALIZING => {
                self.cache
                    .counters
                    .finalization_panics
                    .fetch_add(1, Ordering::Relaxed);

                // Recover shard lock to clean up leaked accounting and in_flight state
                let mut shard = match self.cache.shards[self.shard_idx].lock() {
                    Ok(g) => g,
                    Err(p) => {
                        self.cache
                            .counters
                            .lock_poison_recovered
                            .fetch_add(1, Ordering::Relaxed);
                        p.into_inner()
                    }
                };

                // 1. Clean up active_op_ids if state_instance_id matches
                if let Some(state) = shard.map.get_mut(&self.key) {
                    if state.state_instance_id == self.state_instance_id {
                        let removed = state.active_op_ids.remove(&self.op_id);
                        if removed && state.entry.is_none() && state.active_op_ids.is_empty() {
                            shard.map.remove(&self.key);
                            self.cache
                                .counters
                                .reclamations_total
                                .fetch_add(1, Ordering::Relaxed);
                        }
                    } else {
                        self.cache
                            .counters
                            .stale_ops_ignored
                            .fetch_add(1, Ordering::Relaxed);
                    }
                }

                // 2. Clean up in_flight if matching my op_id
                if let Some(entry) = shard.in_flight.get(&self.key) {
                    if entry.op_id == self.op_id {
                        shard.in_flight.remove(&self.key);
                    }
                }

                drop(shard);

                // 3. Resolve any waiters that were not yet dispatched
                let mut guard = match self.shared_op.waiters.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                let waiters = std::mem::take(&mut *guard);
                drop(guard);
                let mut batch = WaiterBatch::new(waiters, Err(Arc::new(OriginError::Aborted)));
                batch.deliver();
            }
            PHASE_COMPLETED => {}
            _ => unreachable!(),
        }
    }
}
