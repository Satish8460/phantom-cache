use crate::core::coalesce::{
    CoalesceResult, FetchRegistration, FinalizeOutcome, OriginError, SharedOp, WaiterBatch,
};
use crate::intel::diagnostics::{CacheCounters, DiagnosticBuffer};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use serde::Serialize;
use tokio::sync::oneshot;

// ==========================================
// Milestone 1 Types
// ==========================================

pub const MAX_TENANT_ID_LEN: usize = 128;
pub const MAX_PATH_LEN: usize = 2048;
pub const MAX_QUERY_LEN: usize = 4096;
/// Maximum length of the canonical key string. Must equal the cluster frame key cap
/// (`dist::frame::MAX_KEY_LENGTH_BYTES`; equality is pinned by a test).
pub const MAX_CANONICAL_KEY_BYTES: usize = 4096;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CacheKey {
    pub tenant_id: String,
    pub path: String,
    pub query: Option<String>,
}

impl CacheKey {
    pub fn new(tenant_id: Option<&str>, path: &str, query: Option<&str>) -> Self {
        Self {
            tenant_id: tenant_id.unwrap_or("__default__").to_string(),
            path: path.to_string(),
            query: query.map(|q| q.to_string()),
        }
    }

    pub fn validate(&self) -> Result<(), OriginError> {
        if self.tenant_id.len() > MAX_TENANT_ID_LEN {
            return Err(OriginError::TenantIdTooLong);
        }
        // M2: the FIRST ':' of the canonical string must be the tenant separator, so a tenant id
        // is 1..=128 bytes of [A-Za-z0-9._-] (no ':' and nothing needing escaping).
        if self.tenant_id.is_empty()
            || !self
                .tenant_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
        {
            return Err(OriginError::InvalidTenantId);
        }
        if self.path.len() > MAX_PATH_LEN {
            return Err(OriginError::UriTooLong);
        }
        if let Some(ref q) = self.query {
            if q.len() > MAX_QUERY_LEN {
                return Err(OriginError::UriTooLong);
            }
        }
        // H1 (round-2 review): the path and query are written into the upstream request line
        // (`GET <target> HTTP/1.1`). Control characters (C0, DEL) and the space can never appear
        // unescaped in a request target; one in a percent-decoded path (`%0d%0a`, `%20`) would
        // inject headers or a second request at the origin. Reject them at the edge (400). The
        // upstream writer additionally re-encodes the target, so this is defence in depth.
        let bad_byte = |s: &str| s.bytes().any(|b| b <= 0x20 || b == 0x7f);
        if bad_byte(&self.path) || self.query.as_deref().is_some_and(bad_byte) {
            return Err(OriginError::InvalidPath);
        }
        // Round 3: `.` and `..` path segments are rejected (400). An origin normalises them
        // (`/a/../b` == `/b`) while the cache key would treat them as distinct strings, so the
        // same origin resource could be cached under several keys and one key could be made to
        // address another tenant-visible path. A `%2e%2e` decodes to `..` and is rejected too.
        if self.path.split('/').any(|seg| seg == "." || seg == "..") {
            return Err(OriginError::InvalidPath);
        }
        // M2: ":?" in the path is indistinguishable from the path/query separator of the
        // canonical string ("t:/a:?b" = path "/a:?b" OR path "/a" + query "b"). Reject it, so
        // from_canonical_string(to_canonical_string(k)) == k for every valid key. A query may
        // contain ":?" (the FIRST occurrence in the remainder is always the separator).
        if self.path.contains(":?") {
            return Err(OriginError::InvalidPath);
        }
        // M1: the canonical string is what cluster invalidation frames carry; those are capped
        // (dist::frame::MAX_KEY_LENGTH_BYTES). Reject at the edge instead of failing in the
        // cluster layer after the local state has already been changed.
        let canonical_len = self.tenant_id.len()
            + 1
            + self.path.len()
            + self.query.as_ref().map_or(0, |q| 2 + q.len());
        if canonical_len > MAX_CANONICAL_KEY_BYTES {
            return Err(OriginError::UriTooLong);
        }
        Ok(())
    }

    pub fn to_canonical_string(&self) -> String {
        match &self.query {
            Some(q) => format!("{}:{}:?{}", self.tenant_id, self.path, q),
            None => format!("{}:{}", self.tenant_id, self.path),
        }
    }

    pub fn from_canonical_string(s: &str) -> Option<Self> {
        if let Some(first_colon) = s.find(':') {
            let tenant_id = &s[..first_colon];
            let rest = &s[first_colon + 1..];
            let (path, query) = if let Some(query_pos) = rest.find(":?") {
                (&rest[..query_pos], Some(&rest[query_pos + 2..]))
            } else {
                (rest, None)
            };
            Some(Self::new(Some(tenant_id), path, query))
        } else {
            Some(Self::new(None, s, None))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedResource {
    pub status: u16,
    pub content_type: String,
    pub body: Arc<Vec<u8>>,
    pub etag: Option<String>,
    pub created_at_ms: u64,
    pub ttl_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EvictionPolicy {
    Sieve,
    Lru,
    S3Fifo,
}

pub struct KeyState {
    pub state_instance_id: u64,
    pub generation: u64,
    pub active_op_ids: HashSet<u64>,
    pub entry: Option<CachedResource>,
    pub visited: bool,
    pub last_accessed_ms: u64,
    pub access_count: u32,
    pub entry_tenant_epoch: u64,
    pub entry_global_epoch: u64,
    pub freshness: Option<crate::core::freshness::FreshnessMetadata>,
    pub revalidation_backoff_ms: u64,
    pub last_revalidation_failure_ms: u64,
}

pub fn calculate_key_bytes(key: &CacheKey) -> usize {
    key.tenant_id.len() + key.path.len() + key.query.as_ref().map_or(0, |q| q.len())
}

pub fn calculate_resource_bytes(resource: Option<&CachedResource>) -> usize {
    match resource {
        Some(res) => {
            res.body.capacity() + res.content_type.len() + res.etag.as_ref().map_or(0, |e| e.len())
        }
        None => 0,
    }
}

/// RAII reservation of one background-revalidation slot. Dropping it (including during a panic
/// unwind or task cancellation) releases the slot, so `background_refresh_in_flight` cannot leak.
pub struct BackgroundSlot {
    cache: Arc<PartitionedCache>,
}

impl Drop for BackgroundSlot {
    fn drop(&mut self) {
        self.cache
            .counters
            .background_refresh_in_flight
            .fetch_sub(1, Ordering::SeqCst);
    }
}

/// What `PartitionedCache::complete_not_modified` did with a 304.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotModifiedOutcome {
    /// Entry refreshed in place (counted in `revalidation_not_modified`).
    Refreshed,
    /// A refresh was due but could not be applied (fence/epoch); nothing stored.
    NotApplied,
    /// 304 carried a different validator; entry deleted, leader/waiters got a 502.
    ValidatorMismatch,
    /// 304 headers forbid storage; backoff recorded, nothing stored.
    Uncacheable,
    /// No usable resident entry (purged/invalidated meanwhile); nothing stored, 502.
    NoResident,
}

pub fn fixed_entry_overhead() -> usize {
    std::mem::size_of::<CacheKey>() + std::mem::size_of::<KeyState>() + 64
}

pub fn calculate_state_bytes(key: &CacheKey, state: &KeyState) -> usize {
    calculate_key_bytes(key)
        + calculate_resource_bytes(state.entry.as_ref())
        + fixed_entry_overhead()
}

pub struct CacheShard {
    pub map: HashMap<CacheKey, KeyState>,
    pub in_flight: HashMap<CacheKey, Arc<SharedOp>>,
    pub bytes: usize,
    pub byte_budget: usize,
    pub eviction: Box<dyn crate::core::eviction::ShardEvictionPolicy>,
}

impl CacheShard {
    pub fn new() -> Self {
        Self::with_policy(EvictionPolicy::Sieve, 1024)
    }

    pub fn with_policy(policy: EvictionPolicy, capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            in_flight: HashMap::new(),
            bytes: 0,
            byte_budget: usize::MAX,
            eviction: crate::core::eviction::create_shard_eviction_policy(policy, capacity),
        }
    }
}

impl Default for CacheShard {
    fn default() -> Self {
        Self::new()
    }
}

// Legacy entry for backward compatibility with wp1_tests
pub struct CacheEntry {
    pub value: Vec<u8>,
    pub created_at_ms: u64,
    pub ttl_ms: u64,
    pub visited: bool,
}

pub enum FetchRegistrationResult {
    Hit(CachedResource),
    Wait(oneshot::Receiver<CoalesceResult>),
    Initiate(FetchRegistration),
    Rejected(OriginError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaleServingDecision {
    Eligible(CachedResource, crate::core::freshness::FreshnessMetadata),
    CapExceeded,
    Ineligible,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CacheError {
    EpochOverflow,
    GenerationOverflow,
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EpochOverflow => write!(f, "Epoch counter reached u64::MAX and cannot advance"),
            Self::GenerationOverflow => write!(f, "Generation counter reached u64::MAX and cannot advance"),
        }
    }
}

impl std::error::Error for CacheError {}

pub const MAX_TENANT_EPOCHS: usize = 10_000;

pub struct PartitionedCache {
    pub shards: Vec<Mutex<CacheShard>>,
    pub legacy_shards: Vec<Mutex<HashMap<String, CacheEntry>>>,
    pub shard_mask: usize,
    pub counters: Arc<CacheCounters>,
    pub diagnostics: Arc<DiagnosticBuffer>,
    pub next_op_id: AtomicU64,
    pub next_instance_id: AtomicU64,
    pub test_panic_after_waiter_extraction: AtomicBool,
    pub max_entries_per_shard: AtomicUsize,
    pub max_waiters_per_key: AtomicUsize,
    pub eviction_policy: Mutex<EvictionPolicy>,
    pub tenant_epochs: Mutex<HashMap<String, u64>>,
    pub epoch_snapshot: std::sync::RwLock<Arc<HashMap<String, u64>>>,
    pub global_epoch: AtomicU64,
    pub max_stale_ms: AtomicU64,
    pub max_background_refresh: AtomicUsize,
    pub sweeper_enabled: AtomicBool,
    pub sweeper_batch_size: AtomicUsize,
    pub sweeper_interval_ms: AtomicU64,
    /// Number of sweeper loop wake-ups (tick or shutdown-channel event). Observability for D-2.
    pub sweeper_wakeups: AtomicU64,
}

impl PartitionedCache {
    pub fn new(num_shards: usize) -> Self {
        let counters = Arc::new(CacheCounters::new());
        let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
        Self::new_with_diagnostics(num_shards, counters, diagnostics)
    }

    pub fn new_with_diagnostics(
        num_shards: usize,
        counters: Arc<CacheCounters>,
        diagnostics: Arc<DiagnosticBuffer>,
    ) -> Self {
        assert!(
            num_shards.is_power_of_two(),
            "Number of shards must be a power of two"
        );
        let mut shards = Vec::with_capacity(num_shards);
        let mut legacy_shards = Vec::with_capacity(num_shards);
        for _ in 0..num_shards {
            shards.push(Mutex::new(CacheShard::new()));
            legacy_shards.push(Mutex::new(HashMap::new()));
        }
        Self {
            shards,
            legacy_shards,
            shard_mask: num_shards - 1,
            counters,
            diagnostics,
            next_op_id: AtomicU64::new(1),
            next_instance_id: AtomicU64::new(1),
            test_panic_after_waiter_extraction: AtomicBool::new(false),
            max_entries_per_shard: AtomicUsize::new(10_000),
            max_waiters_per_key: AtomicUsize::new(1_000),
            eviction_policy: Mutex::new(EvictionPolicy::Sieve),
            tenant_epochs: Mutex::new(HashMap::new()),
            epoch_snapshot: std::sync::RwLock::new(Arc::new(HashMap::new())),
            global_epoch: AtomicU64::new(0),
            max_stale_ms: AtomicU64::new(86_400_000), // 24 hours (O2-11)
            max_background_refresh: AtomicUsize::new(128),
            sweeper_enabled: AtomicBool::new(true),
            sweeper_batch_size: AtomicUsize::new(32), // <= 32 per O2-8
            sweeper_interval_ms: AtomicU64::new(100),
            sweeper_wakeups: AtomicU64::new(0),
        }
    }

    pub fn get_max_stale_ms(&self) -> u64 {
        self.max_stale_ms.load(Ordering::Relaxed)
    }

    pub fn set_max_stale_ms(&self, ms: u64) {
        self.max_stale_ms.store(ms, Ordering::Relaxed);
    }

    pub fn get_max_background_refresh(&self) -> usize {
        self.max_background_refresh.load(Ordering::Relaxed)
    }

    pub fn set_max_background_refresh(&self, limit: usize) {
        self.max_background_refresh.store(limit, Ordering::Relaxed);
    }

    pub fn is_sweeper_enabled(&self) -> bool {
        self.sweeper_enabled.load(Ordering::Relaxed)
    }

    pub fn set_sweeper_enabled(&self, enabled: bool) {
        self.sweeper_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn get_sweeper_batch_size(&self) -> usize {
        self.sweeper_batch_size.load(Ordering::Relaxed)
    }

    pub fn set_sweeper_batch_size(&self, batch_size: usize) {
        self.sweeper_batch_size.store(batch_size, Ordering::Relaxed);
    }

    pub fn get_sweeper_interval_ms(&self) -> u64 {
        self.sweeper_interval_ms.load(Ordering::Relaxed)
    }

    pub fn set_sweeper_interval_ms(&self, interval_ms: u64) {
        // L-1: tokio::time::interval panics on a zero period; clamp to 1 ms.
        self.sweeper_interval_ms.store(interval_ms.max(1), Ordering::Relaxed);
    }

    pub fn sweep_lazy_reclamation(&self, max_per_shard: usize, now_ms: u64) -> (usize, u64) {
        let mut total_reclaimed = 0usize;
        let mut max_lock_hold_ns = 0u64;
        let hard_cap_ms = self.get_max_stale_ms();

        for shard_mutex in &self.shards {
            let start = std::time::Instant::now();
            let mut shard = match shard_mutex.lock() {
                Ok(g) => g,
                Err(p) => {
                    self.counters.lock_poison_recovered.fetch_add(1, Ordering::Relaxed);
                    p.into_inner()
                }
            };

            let mut candidates_to_reclaim = Vec::new();
            for (key, state) in shard.map.iter() {
                if candidates_to_reclaim.len() >= max_per_shard {
                    break;
                }
                // Skip if active operations or in-flight fetch running
                if !state.active_op_ids.is_empty() || shard.in_flight.contains_key(key) {
                    continue;
                }

                let (tenant_epoch, global_epoch) = self.get_effective_epochs(&key.tenant_id);
                let is_epoch_stale = state.entry_tenant_epoch != tenant_epoch
                    || state.entry_global_epoch != global_epoch;
                // H2: an expired entry is still USABLE while it is inside its stale-while-revalidate
                // / stale-if-error window (unless must-revalidate forbids stale use), bounded by the
                // hard stale cap, exactly as check_stale_serving decides. Retain it until then.
                let is_expired = match &state.entry {
                    Some(res) => {
                        let window_ms = match &state.freshness {
                            Some(m) if !m.must_revalidate => m
                                .stale_while_revalidate_ms
                                .unwrap_or(0)
                                .max(m.stale_if_error_ms.unwrap_or(0))
                                .min(hard_cap_ms),
                            _ => 0,
                        };
                        res.created_at_ms
                            .saturating_add(res.ttl_ms)
                            .saturating_add(window_ms)
                            <= now_ms
                    }
                    None => true,
                };

                if is_epoch_stale || is_expired {
                    candidates_to_reclaim.push(key.clone());
                }
            }

            for key in candidates_to_reclaim {
                if let Some(removed_state) = shard.map.remove(&key) {
                    shard.eviction.on_remove(&key);
                    let bytes = calculate_state_bytes(&key, &removed_state);
                    shard.bytes = shard.bytes.saturating_sub(bytes);
                    self.counters.reclamations_total.fetch_add(1, Ordering::Relaxed);
                    total_reclaimed += 1;
                }
            }

            let elapsed_ns = start.elapsed().as_nanos() as u64;
            if elapsed_ns > max_lock_hold_ns {
                max_lock_hold_ns = elapsed_ns;
            }
        }

        (total_reclaimed, max_lock_hold_ns)
    }

    pub fn spawn_sweeper_task(
        cache: Arc<PartitionedCache>,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut shutdown = shutdown;
            let interval_ms = cache.sweeper_interval_ms.load(Ordering::Relaxed).max(1); // L4: pub field may hold 0
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(interval_ms));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            // D-2: once the watch sender is dropped, `changed()` resolves immediately with Err on
            // every poll. Treat that as "no shutdown channel any more" and stop selecting on it,
            // otherwise the loop busy-spins at 100% of a core while idle.
            let mut shutdown_open = true;
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        cache.sweeper_wakeups.fetch_add(1, Ordering::Relaxed);
                        if cache.sweeper_enabled.load(Ordering::Relaxed) {
                            let batch_size = cache.sweeper_batch_size.load(Ordering::Relaxed);
                            let now_ms = DiagnosticBuffer::now_ms();
                            let _ = cache.sweep_lazy_reclamation(batch_size, now_ms);
                        }
                    }
                    res = shutdown.changed(), if shutdown_open => {
                        cache.sweeper_wakeups.fetch_add(1, Ordering::Relaxed);
                        if res.is_err() {
                            shutdown_open = false;
                        } else if *shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        })
    }

    pub fn get_eviction_policy(&self) -> EvictionPolicy {
        *self.eviction_policy.lock().unwrap()
    }

    pub fn set_eviction_policy(&self, policy: EvictionPolicy) {
        *self.eviction_policy.lock().unwrap() = policy;
        let max_entries = self.max_entries_per_shard.load(Ordering::Relaxed);
        for shard_mutex in &self.shards {
            let mut shard = shard_mutex.lock().unwrap();
            let keys: Vec<CacheKey> = shard.map.keys().cloned().collect();
            let mut new_eviction = crate::core::eviction::create_shard_eviction_policy(policy, max_entries);
            new_eviction.rebuild_from(&keys);
            shard.eviction = new_eviction;
        }
    }

    pub fn set_capacity_limits(&self, max_entries_per_shard: usize, max_waiters_per_key: usize) {
        self.max_entries_per_shard
            .store(max_entries_per_shard, Ordering::Relaxed);
        self.max_waiters_per_key
            .store(max_waiters_per_key, Ordering::Relaxed);

        // Active reconfiguration contraction: immediately evict idle entries down to new bound
        let now_ms = DiagnosticBuffer::now_ms();
        let policy = self.get_eviction_policy();
        for shard_mutex in &self.shards {
            let mut shard = match shard_mutex.lock() {
                Ok(g) => g,
                Err(p) => {
                    self.counters
                        .lock_poison_recovered
                        .fetch_add(1, Ordering::Relaxed);
                    p.into_inner()
                }
            };
            let keys: Vec<CacheKey> = shard.map.keys().cloned().collect();
            let mut new_eviction = crate::core::eviction::create_shard_eviction_policy(policy, max_entries_per_shard);
            new_eviction.rebuild_from(&keys);
            shard.eviction = new_eviction;
            while shard.map.len() > max_entries_per_shard {
                let evicted = Self::evict_from_shard(&mut shard, policy, now_ms, &self.counters);
                if evicted.is_none() {
                    break; // All remaining entries are active operations or in-flight
                }
            }
        }
    }

    pub fn get_shard_index(&self, key: &CacheKey) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) & self.shard_mask
    }

    fn get_legacy_shard_index(&self, key: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) & self.shard_mask
    }

    pub fn get_effective_epochs(&self, tenant_id: &str) -> (u64, u64) {
        let tenant_epoch = {
            let snap = match self.epoch_snapshot.read() {
                Ok(s) => s,
                Err(p) => p.into_inner(),
            };
            snap.get(tenant_id).copied().unwrap_or(0)
        };
        let global_epoch = self.global_epoch.load(Ordering::SeqCst);
        (tenant_epoch, global_epoch)
    }

    pub fn get_tenant_epoch(&self, tenant_id: &str) -> u64 {
        self.get_effective_epochs(tenant_id).0
    }

    pub fn try_invalidate_tenant(&self, tenant_id: &str) -> Result<u64, CacheError> {
        let mut guard = match self.tenant_epochs.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if guard.len() >= MAX_TENANT_EPOCHS && !guard.contains_key(tenant_id) {
            // SEC-4A-01 Remediation: Synchronized Global-Epoch Fallback
            // When capacity is exhausted, increment global_epoch to safely fence
            // all pre-existing cached entries cluster-wide. Clearing the map
            // resets tenant-specific counters without any risk of resurrecting stale entries,
            // because every prior entry has entry_global_epoch < current_global_epoch.
            let current_global = self.global_epoch.load(Ordering::SeqCst);
            let next_global = current_global
                .checked_add(1)
                .ok_or(CacheError::EpochOverflow)?;
            self.global_epoch.store(next_global, Ordering::SeqCst);
            guard.clear();
        }
        let epoch = guard.entry(tenant_id.to_string()).or_insert(0);
        let next_epoch = epoch
            .checked_add(1)
            .ok_or(CacheError::EpochOverflow)?;
        *epoch = next_epoch;
        {
            let mut snap = match self.epoch_snapshot.write() {
                Ok(s) => s,
                Err(p) => p.into_inner(),
            };
            *snap = Arc::new(guard.clone());
        }
        Ok(next_epoch)
    }

    pub fn invalidate_tenant(&self, tenant_id: &str) -> u64 {
        self.try_invalidate_tenant(tenant_id).expect("Tenant epoch counter overflow")
    }

    pub fn try_invalidate_all(&self) -> Result<u64, CacheError> {
        let mut guard = match self.tenant_epochs.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let current_global = self.global_epoch.load(Ordering::SeqCst);
        let next_global = current_global
            .checked_add(1)
            .ok_or(CacheError::EpochOverflow)?;
        self.global_epoch.store(next_global, Ordering::SeqCst);
        guard.clear();
        {
            let mut snap = match self.epoch_snapshot.write() {
                Ok(s) => s,
                Err(p) => p.into_inner(),
            };
            *snap = Arc::new(HashMap::new());
        }
        Ok(next_global)
    }

    pub fn invalidate_all(&self) -> u64 {
        self.try_invalidate_all().expect("Global epoch counter overflow")
    }

    pub fn get_resource(&self, key: &CacheKey, now_ms: u64) -> Option<CachedResource> {
        let idx = self.get_shard_index(key);
        let mut shard = match self.shards[idx].lock() {
            Ok(g) => g,
            Err(p) => {
                self.counters
                    .lock_poison_recovered
                    .fetch_add(1, Ordering::Relaxed);
                p.into_inner()
            }
        };

        let (current_tenant_epoch, current_global_epoch) = self.get_effective_epochs(&key.tenant_id);

        let mut res_to_subtract = 0;
        let mut hit_entry = None;
        if let Some(state) = shard.map.get_mut(key) {
            if let Some(ref entry) = state.entry {
                if state.entry_tenant_epoch == current_tenant_epoch
                    && state.entry_global_epoch == current_global_epoch
                    && entry.created_at_ms.saturating_add(entry.ttl_ms) > now_ms
                {
                    state.visited = true;
                    hit_entry = Some(entry.clone());
                } else if state.entry_tenant_epoch != current_tenant_epoch
                    || state.entry_global_epoch != current_global_epoch
                {
                    res_to_subtract = calculate_resource_bytes(Some(entry));
                    state.entry = None;
                    if let Some(next_gen) = state.generation.checked_add(1) {
                        state.generation = next_gen;
                    }
                }
            }
        }
        if res_to_subtract > 0 {
            shard.bytes = shard.bytes.saturating_sub(res_to_subtract);
        }
        if hit_entry.is_some() {
            return hit_entry;
        }
        None
    }

    pub fn get_freshness(&self, key: &CacheKey) -> Option<crate::core::freshness::FreshnessMetadata> {
        let idx = self.get_shard_index(key);
        let shard = self.shards[idx].lock().ok()?;
        shard.map.get(key).and_then(|s| s.freshness.clone())
    }

    pub fn update_freshness_metadata(
        &self,
        key: &CacheKey,
        metadata: crate::core::freshness::FreshnessMetadata,
    ) {
        let idx = self.get_shard_index(key);
        if let Ok(mut shard) = self.shards[idx].lock() {
            if let Some(state) = shard.map.get_mut(key) {
                state.freshness = Some(metadata);
            }
        }
    }

    pub fn get_resource_with_metadata(
        &self,
        key: &CacheKey,
        now_ms: u64,
    ) -> Option<(CachedResource, Option<crate::core::freshness::FreshnessMetadata>)> {
        let idx = self.get_shard_index(key);
        let mut shard = match self.shards[idx].lock() {
            Ok(g) => g,
            Err(p) => {
                self.counters
                    .lock_poison_recovered
                    .fetch_add(1, Ordering::Relaxed);
                p.into_inner()
            }
        };

        let (current_tenant_epoch, current_global_epoch) = self.get_effective_epochs(&key.tenant_id);

        let mut res_to_subtract = 0;
        let mut hit_entry = None;
        let mut hit_meta = None;
        if let Some(state) = shard.map.get_mut(key) {
            if let Some(ref entry) = state.entry {
                if state.entry_tenant_epoch == current_tenant_epoch
                    && state.entry_global_epoch == current_global_epoch
                    && entry.created_at_ms.saturating_add(entry.ttl_ms) > now_ms
                {
                    state.visited = true;
                    hit_entry = Some(entry.clone());
                    hit_meta = state.freshness.clone();
                } else if state.entry_tenant_epoch != current_tenant_epoch
                    || state.entry_global_epoch != current_global_epoch
                {
                    res_to_subtract = calculate_resource_bytes(Some(entry));
                    state.entry = None;
                    state.freshness = None;
                    if let Some(next_gen) = state.generation.checked_add(1) {
                        state.generation = next_gen;
                    }
                }
            }
        }
        if res_to_subtract > 0 {
            shard.bytes = shard.bytes.saturating_sub(res_to_subtract);
        }
        hit_entry.map(|res| (res, hit_meta))
    }

    pub fn check_stale_serving(
        &self,
        key: &CacheKey,
        now_ms: u64,
        hard_cap_ms: u64,
        is_error_context: bool,
    ) -> StaleServingDecision {
        let idx = self.get_shard_index(key);
        let mut shard = match self.shards[idx].lock() {
            Ok(g) => g,
            Err(p) => {
                self.counters
                    .lock_poison_recovered
                    .fetch_add(1, Ordering::Relaxed);
                p.into_inner()
            }
        };

        let (current_tenant_epoch, current_global_epoch) = self.get_effective_epochs(&key.tenant_id);

        if let Some(state) = shard.map.get_mut(key) {
            if let Some(ref entry) = state.entry {
                // Invalidation check: if epochs mismatch, entry is invalid -> delete wins
                if state.entry_tenant_epoch != current_tenant_epoch
                    || state.entry_global_epoch != current_global_epoch
                {
                    return StaleServingDecision::Ineligible;
                }

                // A fresh entry is not stale: it is served as a normal HIT (same freshness test as
                // get_resource). Without this a fresh entry with an SWR directive was served as
                // STALE and re-triggered background revalidation on every request (S-9).
                if entry.created_at_ms.saturating_add(entry.ttl_ms) > now_ms {
                    return StaleServingDecision::Ineligible;
                }

                if let Some(ref meta) = state.freshness {
                    // Contract (b): must-revalidate forbids ANY stale serving
                    if meta.must_revalidate {
                        return StaleServingDecision::Ineligible;
                    }

                    // Window selection: if origin failed (error context), use stale_if_error, else stale_while_revalidate
                    let window_ms = if is_error_context {
                        meta.stale_if_error_ms.or(meta.stale_while_revalidate_ms).unwrap_or(0)
                    } else {
                        meta.stale_while_revalidate_ms.unwrap_or(0)
                    };

                    if window_ms == 0 {
                        return StaleServingDecision::Ineligible;
                    }

                    let age_ms = now_ms.saturating_sub(entry.created_at_ms);
                    let lifetime_ms = entry.ttl_ms;

                    // Hard staleness cap check (M2-03 invariant)
                    let absolute_hard_cap_ms = lifetime_ms.saturating_add(hard_cap_ms);
                    if age_ms > absolute_hard_cap_ms {
                        return StaleServingDecision::CapExceeded;
                    }

                    // Directive window check bounded by hard cap
                    let effective_cap_ms = lifetime_ms.saturating_add(window_ms.min(hard_cap_ms));
                    if age_ms <= effective_cap_ms {
                        state.visited = true;
                        return StaleServingDecision::Eligible(entry.clone(), meta.clone());
                    }
                }
            }
        }

        StaleServingDecision::Ineligible
    }

    pub fn record_stale_served(&self, age_ms: u64, lifetime_ms: u64, hard_cap_ms: u64) {
        self.counters.stale_served_total.fetch_add(1, Ordering::Relaxed);
        if age_ms > lifetime_ms.saturating_add(hard_cap_ms) {
            self.counters.stale_cap_violations.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn refresh_entry_lifetime(
        &self,
        key: &CacheKey,
        new_created_at_ms: u64,
        new_ttl_ms: u64,
        new_etag: Option<String>,
        metadata: crate::core::freshness::FreshnessMetadata,
    ) {
        let idx = self.get_shard_index(key);
        if let Ok(mut shard) = self.shards[idx].lock() {
            if let Some(state) = shard.map.get_mut(key) {
                if let Some(ref mut entry) = state.entry {
                    entry.created_at_ms = new_created_at_ms;
                    entry.ttl_ms = new_ttl_ms;
                    if new_etag.is_some() {
                        entry.etag = new_etag;
                    }
                }
                state.freshness = Some(metadata);
                state.last_revalidation_failure_ms = 0;
                state.revalidation_backoff_ms = 1000;
            }
        }
    }

    /// Fenced counterpart of `update_freshness_metadata` (S-2). The write is accepted only if the
    /// guard's operation is still the live one for this key: same state instance, same
    /// generation (a DELETE bumps it), and the op is still registered as active. A leader that
    /// was overtaken by a DELETE or a newer fill therefore cannot overwrite newer metadata.
    ///
    /// M1: accepted metadata is NOT written to the resident state here. It is parked on the op
    /// (`SharedOp::pending_freshness`) and installed by `finalize_fetch` atomically with the entry,
    /// under the generation/epoch/budget checks. If the store is rejected (oversize, budget, epoch,
    /// DELETE) or the op ends Uncacheable/Error, the parked metadata is dropped and the resident
    /// entry keeps its own directives. Returns true if accepted by the fence; refusals are counted
    /// in `metadata_setter_rejections`.
    pub fn update_freshness_for_guard(
        &self,
        guard: &crate::core::coalesce::OriginFetchGuard,
        metadata: crate::core::freshness::FreshnessMetadata,
    ) -> bool {
        let idx = self.get_shard_index(&guard.key);
        let live = {
            let shard = match self.shards[idx].lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            shard.map.get(&guard.key).is_some_and(|state| {
                state.state_instance_id == guard.state_instance_id
                    && state.generation == guard.op_gen
                    && state.active_op_ids.contains(&guard.op_id)
            })
        };
        if live {
            let mut slot = match guard.shared_op.pending_freshness.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            *slot = Some(metadata);
            true
        } else {
            self.counters.metadata_setter_rejections.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    /// Fenced counterpart of `refresh_entry_lifetime` (S-1/S-2), for 304 revalidation. Applies
    /// only under the same live-op check as `update_freshness_for_guard`, and only mutates the
    /// resident entry if its epochs are still current. Keeps shard byte accounting exact when
    /// the ETag length changes. Returns true if applied.
    pub fn refresh_entry_lifetime_for_guard(
        &self,
        guard: &crate::core::coalesce::OriginFetchGuard,
        new_created_at_ms: u64,
        new_ttl_ms: u64,
        new_etag: Option<String>,
        metadata: crate::core::freshness::FreshnessMetadata,
    ) -> bool {
        let idx = self.get_shard_index(&guard.key);
        let mut shard = match self.shards[idx].lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let (cur_tenant, cur_global) = self.get_effective_epochs(&guard.key.tenant_id);
        let budget = shard.byte_budget;
        let shard_bytes = shard.bytes;
        let mut byte_delta: (usize, usize) = (0, 0);
        let mut applied = false;
        let mut fence_ok = false;
        if let Some(state) = shard.map.get_mut(&guard.key) {
            // M1: freshness is only written for a live op AND a resident entry whose epochs are
            // still current; otherwise nothing changes.
            fence_ok = state.state_instance_id == guard.state_instance_id
                && state.generation == guard.op_gen
                && state.active_op_ids.contains(&guard.op_id);
            if fence_ok
                && state.entry_tenant_epoch == cur_tenant
                && state.entry_global_epoch == cur_global
            {
                if let Some(ref mut entry) = state.entry {
                    let before = calculate_resource_bytes(Some(entry));
                    let old_etag = entry.etag.clone();
                    entry.created_at_ms = new_created_at_ms;
                    entry.ttl_ms = new_ttl_ms;
                    if new_etag.is_some() {
                        entry.etag = new_etag;
                    }
                    let mut after = calculate_resource_bytes(Some(entry));
                    // Caveat A: an ETag that grows the entry past the shard budget is not adopted.
                    if after > before && shard_bytes.saturating_sub(before).saturating_add(after) > budget {
                        entry.etag = old_etag;
                        after = before;
                    }
                    byte_delta = (before, after);
                    state.freshness = Some(metadata);
                    state.last_revalidation_failure_ms = 0;
                    state.revalidation_backoff_ms = 1000;
                    applied = true;
                }
            }
        }
        if applied {
            shard.bytes = shard.bytes.saturating_sub(byte_delta.0).saturating_add(byte_delta.1);
        } else if fence_ok {
            // Fence passed but the resident entry is missing or epoch-stale.
            self.counters.metadata_refresh_not_applied.fetch_add(1, Ordering::Relaxed);
        } else {
            self.counters.metadata_setter_rejections.fetch_add(1, Ordering::Relaxed);
        }
        applied
    }

    /// Outcome for a revalidation leader that could not refresh the entry (uncacheable 304 or
    /// refresh not applied): the resident body may be handed to the leader and its waiters ONLY if
    /// the normal serving policy allows it now (fresh, or stale inside the SWR/SIE window, not
    /// must-revalidate, within the hard stale cap, epochs current). The state is re-read here, so a
    /// DELETE between peek and refresh is honoured. Otherwise a 502 is returned and no body leaks.
    /// Stale hand-outs are counted via `record_stale_served` like every other stale path.
    fn serveable_outcome(&self, key: &CacheKey, now_ms: u64) -> FinalizeOutcome {
        if let Some((fresh, _)) = self.get_resource_with_metadata(key, now_ms) {
            return FinalizeOutcome::Uncacheable(Arc::new(fresh));
        }
        let cap = self.get_max_stale_ms();
        match self.check_stale_serving(key, now_ms, cap, true) {
            StaleServingDecision::Eligible(r, _) => {
                self.record_stale_served(now_ms.saturating_sub(r.created_at_ms), r.ttl_ms, cap);
                FinalizeOutcome::Uncacheable(Arc::new(r))
            }
            _ => FinalizeOutcome::Error(Arc::new(OriginError::UpstreamStatus(502))),
        }
    }

    /// Atomically reserves a background-revalidation slot if fewer than `max` are in flight
    /// (compare-and-increment; no check-then-increment window). `None` = at the bound.
    pub fn try_reserve_background_slot(self: &Arc<Self>, max: u64) -> Option<BackgroundSlot> {
        self.counters
            .background_refresh_in_flight
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |c| if c < max { Some(c + 1) } else { None })
            .ok()
            .map(|_| BackgroundSlot { cache: self.clone() })
    }

    /// Clone of the resident entry and its freshness metadata, REGARDLESS of expiry, but only if
    /// its tenant/global epochs are current. Used by 304 handling, which must act on the state
    /// present AFTER single-flight registration, never on a copy captured before it (H1).
    pub fn peek_resident(
        &self,
        key: &CacheKey,
    ) -> Option<(CachedResource, Option<crate::core::freshness::FreshnessMetadata>)> {
        let idx = self.get_shard_index(key);
        let shard = match self.shards[idx].lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let (t, g) = self.get_effective_epochs(&key.tenant_id);
        shard.map.get(key).and_then(|s| {
            if s.entry_tenant_epoch == t && s.entry_global_epoch == g {
                s.entry.as_ref().map(|e| (e.clone(), s.freshness.clone()))
            } else {
                None
            }
        })
    }

    /// Finishes a revalidation leader that received a `304 Not Modified` (RFC 9111 4.3.4) and
    /// completes the guard. All decisions use the state resident NOW (after registration):
    ///  - nothing resident / epoch-stale: complete with a 502 error, store nothing (H1: a purged
    ///    entry is never resurrected from a copy captured earlier);
    ///  - refresh refused by the fence or epoch check: complete `Uncacheable` (nothing stored);
    ///  - refresh applied: complete `Success` with the entry AS ADOPTED by the refresh (so an ETag
    ///    skipped by the byte-budget guard is not re-introduced by finalize, L-4);
    ///  - validator mismatch: delete the entry, 502 for the leader and any waiters (never a 304
    ///    they did not ask for, M-a);
    ///  - uncacheable 304: back off, `Uncacheable` (nothing stored).
    pub fn complete_not_modified(
        &self,
        guard: crate::core::coalesce::OriginFetchGuard,
        headers_304: &[(Option<axum::http::header::HeaderName>, axum::http::HeaderValue)],
        now_ms: u64,
        has_auth: bool,
    ) -> NotModifiedOutcome {
        use crate::core::freshness::{merge_not_modified, NotModifiedMerge};
        let bad_gateway = || FinalizeOutcome::Error(Arc::new(OriginError::UpstreamStatus(502)));
        let Some((resident, stored_meta)) = self.peek_resident(&guard.key) else {
            self.counters.revalidation_not_applied.fetch_add(1, Ordering::Relaxed);
            if let Some(mut b) = guard.complete(bad_gateway()) {
                b.deliver();
            }
            return NotModifiedOutcome::NoResident;
        };
        let key = guard.key.clone();
        let (outcome, result) = match merge_not_modified(
            resident.ttl_ms,
            resident.etag.as_deref(),
            stored_meta.as_ref(),
            headers_304,
            now_ms,
            has_auth,
        ) {
            NotModifiedMerge::Refresh { ttl_ms, etag, meta } => {
                if self.refresh_entry_lifetime_for_guard(&guard, now_ms, ttl_ms, etag, meta) {
                    match self.peek_resident(&key) {
                        Some((adopted, _)) => {
                            self.counters.revalidation_not_modified.fetch_add(1, Ordering::Relaxed);
                            (FinalizeOutcome::Success(Arc::new(adopted)), NotModifiedOutcome::Refreshed)
                        }
                        None => {
                            self.counters.revalidation_not_applied.fetch_add(1, Ordering::Relaxed);
                            (self.serveable_outcome(&key, now_ms), NotModifiedOutcome::NotApplied)
                        }
                    }
                } else {
                    self.counters.revalidation_not_applied.fetch_add(1, Ordering::Relaxed);
                    (self.serveable_outcome(&key, now_ms), NotModifiedOutcome::NotApplied)
                }
            }
            NotModifiedMerge::ValidatorMismatch => {
                self.counters.revalidation_error.fetch_add(1, Ordering::Relaxed);
                self.delete_if_guard_current(&guard);
                (bad_gateway(), NotModifiedOutcome::ValidatorMismatch)
            }
            NotModifiedMerge::Uncacheable => {
                self.counters.revalidation_error.fetch_add(1, Ordering::Relaxed);
                self.record_revalidation_failure(&key, DiagnosticBuffer::now_ms());
                (self.serveable_outcome(&key, now_ms), NotModifiedOutcome::Uncacheable)
            }
        };
        if let Some(mut b) = guard.complete(outcome) {
            b.deliver();
        }
        result
    }

    pub fn record_revalidation_failure(&self, key: &CacheKey, now_ms: u64) {
        let idx = self.get_shard_index(key);
        {
            // L5: recover a poisoned shard like the neighbouring code instead of skipping.
            let mut shard = match self.shards[idx].lock() {
                Ok(g) => g,
                Err(p) => {
                    self.counters.lock_poison_recovered.fetch_add(1, Ordering::Relaxed);
                    p.into_inner()
                }
            };
            if let Some(state) = shard.map.get_mut(key) {
                if state.last_revalidation_failure_ms == 0 {
                    state.revalidation_backoff_ms = 1000;
                } else {
                    state.revalidation_backoff_ms =
                        (state.revalidation_backoff_ms.saturating_mul(2)).min(60_000);
                }
                state.last_revalidation_failure_ms = now_ms;
            }
        }
    }

    pub fn is_revalidation_due(&self, key: &CacheKey, now_ms: u64) -> bool {
        let idx = self.get_shard_index(key);
        {
            let shard = match self.shards[idx].lock() {
                Ok(g) => g,
                Err(p) => {
                    self.counters.lock_poison_recovered.fetch_add(1, Ordering::Relaxed);
                    p.into_inner()
                }
            };
            if let Some(state) = shard.map.get(key) {
                if state.last_revalidation_failure_ms > 0
                    && now_ms
                        < state
                            .last_revalidation_failure_ms
                            .saturating_add(state.revalidation_backoff_ms)
                {
                    return false;
                }
            }
        }
        true
    }

    pub fn evict_sieve_from_shard(
        shard: &mut CacheShard,
        now_ms: u64,
        counters: &CacheCounters,
    ) -> Option<CacheKey> {
        Self::evict_sieve_from_shard_excluding(shard, now_ms, counters, None)
    }

    pub fn evict_sieve_from_shard_excluding(
        shard: &mut CacheShard,
        _now_ms: u64,
        counters: &CacheCounters,
        exclude_key: Option<&CacheKey>,
    ) -> Option<CacheKey> {
        Self::evict_with_policy(shard, EvictionPolicy::Sieve, counters, exclude_key)
    }

    pub fn evict_lru_from_shard(
        shard: &mut CacheShard,
        now_ms: u64,
        counters: &CacheCounters,
    ) -> Option<CacheKey> {
        Self::evict_lru_from_shard_excluding(shard, now_ms, counters, None)
    }

    pub fn evict_lru_from_shard_excluding(
        shard: &mut CacheShard,
        _now_ms: u64,
        counters: &CacheCounters,
        exclude_key: Option<&CacheKey>,
    ) -> Option<CacheKey> {
        Self::evict_with_policy(shard, EvictionPolicy::Lru, counters, exclude_key)
    }

    pub fn evict_s3fifo_from_shard(
        shard: &mut CacheShard,
        now_ms: u64,
        counters: &CacheCounters,
    ) -> Option<CacheKey> {
        Self::evict_s3fifo_from_shard_excluding(shard, now_ms, counters, None)
    }

    pub fn evict_s3fifo_from_shard_excluding(
        shard: &mut CacheShard,
        _now_ms: u64,
        counters: &CacheCounters,
        exclude_key: Option<&CacheKey>,
    ) -> Option<CacheKey> {
        Self::evict_with_policy(shard, EvictionPolicy::S3Fifo, counters, exclude_key)
    }

    pub fn evict_from_shard(
        shard: &mut CacheShard,
        policy: EvictionPolicy,
        now_ms: u64,
        counters: &CacheCounters,
    ) -> Option<CacheKey> {
        Self::evict_from_shard_excluding(shard, policy, now_ms, counters, None)
    }

    pub fn evict_from_shard_excluding(
        shard: &mut CacheShard,
        policy: EvictionPolicy,
        _now_ms: u64,
        counters: &CacheCounters,
        exclude_key: Option<&CacheKey>,
    ) -> Option<CacheKey> {
        Self::evict_with_policy(shard, policy, counters, exclude_key)
    }

    fn evict_with_policy(
        shard: &mut CacheShard,
        policy: EvictionPolicy,
        counters: &CacheCounters,
        exclude_key: Option<&CacheKey>,
    ) -> Option<CacheKey> {
        let expected_name = match policy {
            EvictionPolicy::Sieve => "sieve",
            EvictionPolicy::Lru => "lru",
            EvictionPolicy::S3Fifo => "s3fifo",
        };
        if shard.eviction.name() != expected_name {
            let keys: Vec<CacheKey> = shard.map.keys().cloned().collect();
            let mut new_eviction = crate::core::eviction::create_shard_eviction_policy(policy, 1024);
            new_eviction.rebuild_from(&keys);
            shard.eviction = new_eviction;
        }

        let mut is_pinned = |k: &CacheKey| -> bool {
            Some(k) == exclude_key
                || shard.in_flight.contains_key(k)
                || shard.map.get(k).map(|s| !s.active_op_ids.is_empty()).unwrap_or(true)
        };

        let victim = shard.eviction.select_victim(&mut is_pinned);

        if let Some(ref k) = victim {
            if let Some(removed_state) = shard.map.remove(k) {
                let entry_bytes = calculate_state_bytes(k, &removed_state);
                shard.bytes = shard.bytes.saturating_sub(entry_bytes);
            }
            counters.evictions_total.fetch_add(1, Ordering::Relaxed);
        }

        victim
    }

    pub fn begin_fetch_atomic(
        self: &Arc<Self>,
        key: &CacheKey,
        now_ms: u64,
    ) -> FetchRegistrationResult {
        let idx = self.get_shard_index(key);
        let mut shard = match self.shards[idx].lock() {
            Ok(g) => g,
            Err(p) => {
                self.counters
                    .lock_poison_recovered
                    .fetch_add(1, Ordering::Relaxed);
                p.into_inner()
            }
        };

        // 1. Check cache hit
        let (current_tenant_epoch, current_global_epoch) = self.get_effective_epochs(&key.tenant_id);

        let mut res_to_subtract = 0;
        let mut hit_entry = None;
        if let Some(state) = shard.map.get_mut(key) {
            if let Some(ref entry) = state.entry {
                if state.entry_tenant_epoch == current_tenant_epoch
                    && state.entry_global_epoch == current_global_epoch
                    && entry.created_at_ms.saturating_add(entry.ttl_ms) > now_ms
                {
                    state.visited = true;
                    state.last_accessed_ms = now_ms;
                    state.access_count = state.access_count.saturating_add(1);
                    hit_entry = Some(entry.clone());
                } else if state.entry_tenant_epoch != current_tenant_epoch
                    || state.entry_global_epoch != current_global_epoch
                {
                    res_to_subtract = calculate_resource_bytes(Some(entry));
                    state.entry = None;
                    if let Some(next_gen) = state.generation.checked_add(1) {
                        state.generation = next_gen;
                    }
                }
            }
        }
        if res_to_subtract > 0 {
            shard.bytes = shard.bytes.saturating_sub(res_to_subtract);
        }
        if let Some(entry) = hit_entry {
            shard.eviction.on_hit(key);
            self.counters.cache_hits.fetch_add(1, Ordering::Relaxed);
            return FetchRegistrationResult::Hit(entry);
        }

        // 2. Check if already in-flight
        // L3 (documented, NOT fixed): a request arriving after try_invalidate_tenant/_all can still
        // join a fetch that started BEFORE the invalidation (SharedOp records no epoch) and receive its
        // pre-invalidation body. finalize_fetch's epoch check keeps that body OUT of the cache, so the
        // exposure is limited to the coalesced waiters of that one in-flight op. Fix would need an
        // epoch snapshot on SharedOp and a detach-on-invalidate; deferred (behaviour change).
        if let Some(existing_op) = shard.in_flight.get(key) {
            let mut guard = match existing_op.waiters.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            let max_waiters = self.max_waiters_per_key.load(Ordering::Relaxed);
            if guard.len() >= max_waiters {
                self.counters
                    .waiter_limit_rejections
                    .fetch_add(1, Ordering::Relaxed);
                return FetchRegistrationResult::Rejected(OriginError::WaiterLimitExceeded);
            }

            let (tx, rx) = oneshot::channel();
            guard.push(tx);
            self.counters
                .coalesced_waiters
                .fetch_add(1, Ordering::Relaxed);
            return FetchRegistrationResult::Wait(rx);
        }

        // 3. Cache miss: First requester initiates origin fetch
        let key_is_present = shard.map.contains_key(key);
        let needed_bytes = if key_is_present {
            0
        } else {
            calculate_key_bytes(key) + fixed_entry_overhead()
        };

        let policy = self.get_eviction_policy();
        let max_entries = self.max_entries_per_shard.load(Ordering::Relaxed);
        let mut evictions_done = 0;

        while evictions_done < 32 && (
            (!key_is_present && shard.map.len() >= max_entries) ||
            shard.bytes.saturating_add(needed_bytes) > shard.byte_budget
        ) {
            if Self::evict_from_shard_excluding(&mut shard, policy, now_ms, &self.counters, Some(key)).is_some() {
                evictions_done += 1;
            } else {
                break; // No more evictable entries
            }
        }

        if (!shard.map.contains_key(key) && shard.map.len() >= max_entries) ||
           shard.bytes.saturating_add(needed_bytes) > shard.byte_budget {
            self.counters
                .capacity_rejections
                .fetch_add(1, Ordering::Relaxed);
            return FetchRegistrationResult::Rejected(OriginError::CapacityExceeded);
        }

        self.counters.cache_misses.fetch_add(1, Ordering::Relaxed);
        let op_id = self.next_op_id.fetch_add(1, Ordering::Relaxed);

        let next_inst = self.next_instance_id.fetch_add(1, Ordering::Relaxed);
        if !key_is_present {
            shard.bytes = shard.bytes.saturating_add(needed_bytes);
        }
        let state = shard.map.entry(key.clone()).or_insert_with(|| {
            KeyState {
                state_instance_id: next_inst,
                generation: 0,
                active_op_ids: HashSet::new(),
                entry: None,
                visited: true,
                last_accessed_ms: now_ms,
                access_count: 1,
                entry_tenant_epoch: current_tenant_epoch,
                entry_global_epoch: current_global_epoch,
                freshness: None,
                revalidation_backoff_ms: 1000,
                last_revalidation_failure_ms: 0,
            }
        });
        state.entry_tenant_epoch = current_tenant_epoch;
        state.entry_global_epoch = current_global_epoch;
        state.active_op_ids.insert(op_id);
        let op_gen = state.generation;
        let state_instance_id = state.state_instance_id;

        let shared_op = Arc::new(SharedOp {
            op_id,
            op_gen,
            state_instance_id,
            waiters: Mutex::new(Vec::new()),
            pending_freshness: Mutex::new(None),
        });

        shard.in_flight.insert(key.clone(), shared_op.clone());

        let reg = FetchRegistration {
            cache: self.clone(),
            key: key.clone(),
            shard_idx: idx,
            op_id,
            op_gen,
            state_instance_id,
            shared_op,
            disarmed: false,
        };

        FetchRegistrationResult::Initiate(reg)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finalize_fetch(
        &self,
        key: &CacheKey,
        shard_idx: usize,
        op_id: u64,
        op_gen: u64,
        state_instance_id: u64,
        shared_op: &Arc<SharedOp>,
        outcome: FinalizeOutcome,
    ) -> Option<WaiterBatch> {
        let mut shard = match self.shards[shard_idx].lock() {
            Ok(g) => g,
            Err(poisoned) => {
                self.counters
                    .lock_poison_recovered
                    .fetch_add(1, Ordering::Relaxed);
                poisoned.into_inner()
            }
        };

        // COMMIT BOUNDARY 1: Detach in_flight entry and extract own waiters
        if let Some(entry) = shard.in_flight.get(key) {
            if entry.op_id == op_id {
                shard.in_flight.remove(key);
            }
        }

        let waiters = {
            let mut guard = match shared_op.waiters.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            std::mem::take(&mut *guard)
        };

        let waiter_batch = WaiterBatch::new(
            waiters,
            match &outcome {
                FinalizeOutcome::Success(r) => Ok(Arc::clone(r)),
                FinalizeOutcome::Uncacheable(r) => Ok(Arc::clone(r)),
                FinalizeOutcome::Error(e) => Err(Arc::clone(e)),
            },
        );

        // Injected failure for testing partial-finalization boundary (M1-T23)
        if self
            .test_panic_after_waiter_extraction
            .load(Ordering::Relaxed)
        {
            panic!("injected panic after waiter extraction");
        }

        // COMMIT BOUNDARY 2: Update cache state and lifecycle accounting
        let mut should_reclaim_placeholder = false;
        let mut store_resource: Option<(CachedResource, usize, usize)> = None;

        if let Some(state) = shard.map.get_mut(key) {
            if state.state_instance_id == state_instance_id {
                if state.active_op_ids.remove(&op_id) {
                    match &outcome {
                        FinalizeOutcome::Success(resource) => {
                            let (current_tenant_epoch, current_global_epoch) = self.get_effective_epochs(&key.tenant_id);
                            if state.generation == op_gen
                                && state.entry_tenant_epoch == current_tenant_epoch
                                && state.entry_global_epoch == current_global_epoch
                            {
                                let old_res_bytes = calculate_resource_bytes(state.entry.as_ref());
                                let new_res_bytes = calculate_resource_bytes(Some(resource));
                                store_resource = Some(((**resource).clone(), old_res_bytes, new_res_bytes));
                            } else {
                                self.counters
                                    .stale_writes_prevented
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        FinalizeOutcome::Uncacheable(_) | FinalizeOutcome::Error(_) => {}
                    }

                    if state.entry.is_none() && state.active_op_ids.is_empty() {
                        should_reclaim_placeholder = true;
                    }
                }
            } else {
                self.counters
                    .stale_ops_ignored
                    .fetch_add(1, Ordering::Relaxed);
            }
        }

        if let Some((resource, old_res_bytes, new_res_bytes)) = store_resource {
            let key_bytes = calculate_key_bytes(key);
            let fixed_overhead = fixed_entry_overhead();
            let total_entry_bytes = key_bytes + fixed_overhead + new_res_bytes;
            let delta_add = new_res_bytes.saturating_sub(old_res_bytes);

            let is_oversize = total_entry_bytes > shard.byte_budget;
            if is_oversize {
                self.counters.byte_budget_rejections.fetch_add(1, Ordering::Relaxed);
                self.counters.oversize_rejections.fetch_add(1, Ordering::Relaxed);
            } else {
                if shard.bytes.saturating_add(delta_add) > shard.byte_budget {
                    let policy = self.get_eviction_policy();
                    let mut evictions = 0;
                    while evictions < 32 && shard.bytes.saturating_add(delta_add) > shard.byte_budget {
                        if Self::evict_from_shard_excluding(&mut shard, policy, resource.created_at_ms, &self.counters, Some(key)).is_some() {
                            evictions += 1;
                        } else {
                            break;
                        }
                    }
                }

                if shard.bytes.saturating_add(delta_add) > shard.byte_budget {
                    self.counters.byte_budget_rejections.fetch_add(1, Ordering::Relaxed);
                } else {
                    shard.bytes = shard.bytes.saturating_sub(old_res_bytes).saturating_add(new_res_bytes);
                    let (current_tenant_epoch, current_global_epoch) = self.get_effective_epochs(&key.tenant_id);
                    let mut inserted = false;
                    let pending_meta = match shared_op.pending_freshness.lock() {
                        Ok(mut g) => g.take(),
                        Err(p) => p.into_inner().take(),
                    };
                    if let Some(state) = shard.map.get_mut(key) {
                        if pending_meta.is_some() {
                            // M1: metadata is installed together with the entry it describes.
                            state.freshness = pending_meta;
                        }
                        state.entry = Some(resource);
                        state.visited = true;
                        state.last_accessed_ms = state.entry.as_ref().map_or(0, |e| e.created_at_ms);
                        state.access_count = state.access_count.max(1);
                        state.entry_tenant_epoch = current_tenant_epoch;
                        state.entry_global_epoch = current_global_epoch;
                        inserted = true;
                    }
                    if inserted {
                        shard.eviction.on_insert(key);
                    }
                    should_reclaim_placeholder = false;
                }
            }
        }

        if should_reclaim_placeholder {
            if let Some(removed_state) = shard.map.remove(key) {
                shard.eviction.on_remove(key);
                let key_fixed = calculate_state_bytes(key, &removed_state);
                shard.bytes = shard.bytes.saturating_sub(key_fixed);
            }
            self.counters
                .reclamations_total
                .fetch_add(1, Ordering::Relaxed);
        }

        // Parked metadata that was not installed above (store rejected, or Uncacheable/Error).
        {
            let mut slot = match shared_op.pending_freshness.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            if slot.take().is_some() {
                self.counters.pending_metadata_dropped.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Shrink step (I8 recovery / convergence)
        let now_ms = DiagnosticBuffer::now_ms();
        let policy = self.get_eviction_policy();
        Self::shrink_shard(&mut shard, policy, now_ms, &self.counters);

        drop(shard);
        Some(waiter_batch)
    }

    pub fn delete(&self, key: &CacheKey) {
        self.delete_impl(key, None);
    }

    /// Fenced delete for an in-flight leader (M2): removes the entry only if the key's state
    /// instance and generation still equal the guard's. A leader overtaken by a DELETE and a newer
    /// fill therefore cannot purge the newer entry. Returns true if it deleted.
    pub fn delete_if_guard_current(&self, guard: &crate::core::coalesce::OriginFetchGuard) -> bool {
        self.delete_impl(&guard.key, Some((guard.state_instance_id, guard.op_gen)))
    }

    fn delete_impl(&self, key: &CacheKey, only_if: Option<(u64, u64)>) -> bool {
        let idx = self.get_shard_index(key);
        let mut shard = match self.shards[idx].lock() {
            Ok(g) => g,
            Err(p) => {
                self.counters
                    .lock_poison_recovered
                    .fetch_add(1, Ordering::Relaxed);
                p.into_inner()
            }
        };
        if let Some((instance, generation)) = only_if {
            let current = shard
                .map
                .get(key)
                .is_some_and(|s| s.state_instance_id == instance && s.generation == generation);
            if !current {
                return false;
            }
        }

        let mut res_to_subtract = 0;
        let mut should_remove = false;
        if let Some(state) = shard.map.get_mut(key) {
            if let Some(ref entry) = state.entry {
                res_to_subtract = calculate_resource_bytes(Some(entry));
            }
            state.entry = None;
            state.freshness = None;
            if let Some(next_gen) = state.generation.checked_add(1) {
                state.generation = next_gen;
            }
            if state.active_op_ids.is_empty() {
                should_remove = true;
            }
        }
        if res_to_subtract > 0 {
            shard.bytes = shard.bytes.saturating_sub(res_to_subtract);
        }
        if should_remove {
            if let Some(removed_state) = shard.map.remove(key) {
                shard.eviction.on_remove(key);
                let key_fixed = calculate_state_bytes(key, &removed_state);
                shard.bytes = shard.bytes.saturating_sub(key_fixed);
            }
            self.counters
                .reclamations_total
                .fetch_add(1, Ordering::Relaxed);
        }

        // Detach in-flight entry so post-DELETE GETs do not join pre-DELETE fetch
        let _old_op = shard.in_flight.remove(key);

        // Shrink step (I8 recovery / convergence)
        let now_ms = DiagnosticBuffer::now_ms();
        let policy = self.get_eviction_policy();
        Self::shrink_shard(&mut shard, policy, now_ms, &self.counters);
        true
    }

    pub fn try_delete_with_status(&self, key: &CacheKey) -> Result<(bool, u32), CacheError> {
        let idx = self.get_shard_index(key);
        let mut shard = match self.shards[idx].lock() {
            Ok(g) => g,
            Err(p) => {
                self.counters
                    .lock_poison_recovered
                    .fetch_add(1, Ordering::Relaxed);
                p.into_inner()
            }
        };

        let mut was_present = false;
        let mut reclaimed_bytes = 0u32;
        let mut res_to_subtract = 0;
        let mut should_remove = false;

        if let Some(state) = shard.map.get_mut(key) {
            if let Some(ref entry) = state.entry {
                was_present = true;
                reclaimed_bytes = entry.body.len() as u32;
                res_to_subtract = calculate_resource_bytes(Some(entry));
            }
            state.entry = None;
            state.freshness = None;
            let next_gen = state
                .generation
                .checked_add(1)
                .ok_or(CacheError::GenerationOverflow)?;
            state.generation = next_gen;
            if state.active_op_ids.is_empty() {
                should_remove = true;
            }
        }
        if res_to_subtract > 0 {
            shard.bytes = shard.bytes.saturating_sub(res_to_subtract);
        }
        if should_remove {
            if let Some(removed_state) = shard.map.remove(key) {
                shard.eviction.on_remove(key);
                let key_fixed = calculate_state_bytes(key, &removed_state);
                shard.bytes = shard.bytes.saturating_sub(key_fixed);
            }
            self.counters
                .reclamations_total
                .fetch_add(1, Ordering::Relaxed);
        }

        let _old_op = shard.in_flight.remove(key);

        // Shrink step (I8 recovery / convergence)
        let now_ms = DiagnosticBuffer::now_ms();
        let policy = self.get_eviction_policy();
        Self::shrink_shard(&mut shard, policy, now_ms, &self.counters);
        Ok((was_present, reclaimed_bytes))
    }

    pub fn delete_with_status(&self, key: &CacheKey) -> (bool, u32) {
        self.try_delete_with_status(key).unwrap_or((false, 0))
    }

    pub fn try_delete_canonical(&self, canonical_key: &str) -> Result<(bool, u32), CacheError> {
        let (mut was_present, mut reclaimed_bytes) = if let Some(cache_key) = CacheKey::from_canonical_string(canonical_key) {
            self.try_delete_with_status(&cache_key)?
        } else {
            (false, 0)
        };

        let idx_legacy = self.get_legacy_shard_index(canonical_key);
        let mut legacy_shard = self.legacy_shards[idx_legacy].lock().unwrap();
        if let Some(e) = legacy_shard.remove(canonical_key) {
            was_present = true;
            reclaimed_bytes = reclaimed_bytes.saturating_add(e.value.len() as u32);
        }

        Ok((was_present, reclaimed_bytes))
    }

    pub fn delete_canonical(&self, canonical_key: &str) -> (bool, u32) {
        self.try_delete_canonical(canonical_key).unwrap_or((false, 0))
    }

    // ==========================================
    // Backward compatibility methods for existing unit tests
    // ==========================================

    pub fn get(&self, key: &str, now_ms: u64) -> Option<Vec<u8>> {
        let idx = self.get_legacy_shard_index(key);
        let mut shard = self.legacy_shards[idx].lock().unwrap();
        if let Some(entry) = shard.get_mut(key) {
            if entry.created_at_ms.saturating_add(entry.ttl_ms) > now_ms {
                entry.visited = true;
                return Some(entry.value.clone());
            }
        }
        None
    }

    pub fn insert(&self, key: &str, value: Vec<u8>, ttl_ms: u64, now_ms: u64) {
        let idx = self.get_legacy_shard_index(key);
        let mut shard = self.legacy_shards[idx].lock().unwrap();
        shard.insert(
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
        for shard_mutex in &self.legacy_shards {
            let mut shard = shard_mutex.lock().unwrap();
            let mut victim_key: Option<String> = None;

            for (k, entry) in shard.iter_mut() {
                if entry.created_at_ms.saturating_add(entry.ttl_ms) <= now_ms {
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
                shard.remove(k);
                return Some(k.clone());
            }
        }
        None
    }

    pub fn shrink_shard(
        shard: &mut CacheShard,
        policy: EvictionPolicy,
        now_ms: u64,
        counters: &CacheCounters,
    ) {
        let mut evictions = 0;
        while evictions < 256 && shard.bytes > shard.byte_budget {
            if Self::evict_from_shard_excluding(shard, policy, now_ms, counters, None).is_some() {
                evictions += 1;
            } else {
                break;
            }
        }
    }

    pub fn bytes_resident(&self) -> usize {
        let mut total = 0usize;
        for shard_mutex in &self.shards {
            let shard = match shard_mutex.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            total = total.saturating_add(shard.bytes);
        }
        total
    }

    pub fn byte_budget(&self) -> usize {
        let mut total = 0usize;
        for shard_mutex in &self.shards {
            let shard = match shard_mutex.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            if shard.byte_budget == usize::MAX {
                return usize::MAX;
            }
            total = total.saturating_add(shard.byte_budget);
        }
        total
    }

    pub fn audit_bytes(&self) -> (usize, usize) {
        let mut accounted = 0usize;
        let mut recorded = 0usize;
        for shard_mutex in &self.shards {
            let shard = match shard_mutex.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            recorded = recorded.saturating_add(shard.bytes);
            for (k, state) in &shard.map {
                accounted = accounted.saturating_add(calculate_state_bytes(k, state));
            }
        }
        (accounted, recorded)
    }

    pub fn set_byte_budget(&self, total_bytes: usize) {
        let num_shards = self.shards.len();
        let shard_budget = if total_bytes == usize::MAX {
            usize::MAX
        } else {
            total_bytes.checked_div(num_shards).unwrap_or(total_bytes)
        };

        let now_ms = DiagnosticBuffer::now_ms();
        let policy = self.get_eviction_policy();

        for shard_mutex in &self.shards {
            let mut shard = match shard_mutex.lock() {
                Ok(g) => g,
                Err(p) => {
                    self.counters
                        .lock_poison_recovered
                        .fetch_add(1, Ordering::Relaxed);
                    p.into_inner()
                }
            };

            shard.byte_budget = shard_budget;

            while shard.bytes > shard.byte_budget {
                let evicted = Self::evict_from_shard_excluding(&mut shard, policy, now_ms, &self.counters, None);
                if evicted.is_none() {
                    break;
                }
            }
        }
    }
}

pub fn audit_shard_bytes(shard: &CacheShard) -> (usize, usize) {
    let mut accounted = 0usize;
    for (k, state) in &shard.map {
        accounted = accounted.saturating_add(calculate_state_bytes(k, state));
    }
    (accounted, shard.bytes)
}
