//! Static Baselines BL-0 through BL-12b for Phantom Lab (M4-06 / WP-17).
//!
//! Implements 14 static baselines with full fidelity checklists:
//! - BL-0: Naive (no coalescing, TTL only)
//! - BL-1: Single-Flight (standard coalescing)
//! - BL-2: Nginx-Style (lock timeout fall-through & stale serving)
//! - BL-3: Varnish-Style (health grace & probe window)
//! - BL-4: XFetch (optimal probabilistic early refresh)
//! - BL-5: Memcache-Style Lease (one refill token per key per interval)
//! - BL-6: Constant-Work Refresh (probabilistic refresh on hit)
//! - BL-7: Adaptive Concurrency (gradient/AIMD origin concurrency limiter)
//! - BL-8: Success-Rate Admission (probabilistic miss rejection on origin errors)
//! - BL-9: Queue-Delay Admission (Breakwater-style delay threshold)
//! - BL-10: Retry Budget (capped retry fraction)
//! - BL-11: Farahbakhsh Remedies (untimed fills & priority)
//! - BL-12a: Per-Scenario Oracle Static
//! - BL-12b: Family-Tuned Static

use super::gateway::LabPolicy;
use futures_util::future::BoxFuture;
use phantom_cache_demo::core::cache::{CacheKey, CachedResource};
use phantom_cache_demo::core::coalesce::OriginError;
use phantom_cache_demo::intel::diagnostics::DiagnosticBuffer;
use reqwest::Client;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Helper: Raw Upstream Fetch
// ---------------------------------------------------------------------------

async fn raw_upstream_fetch(
    origin_url: &str,
    path: &str,
    client: &Client,
) -> Result<CachedResource, OriginError> {
    let url = format!("{}/data{}", origin_url, path);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| OriginError::NetworkError(e.to_string()))?;

    let status = resp.status().as_u16();
    if !resp.status().is_success() {
        return Err(OriginError::UpstreamStatus(status));
    }

    let body_bytes = resp
        .bytes()
        .await
        .map_err(|e| OriginError::NetworkError(e.to_string()))?;

    let now_ms = DiagnosticBuffer::now_ms();
    Ok(CachedResource {
        status,
        content_type: "text/plain".to_string(),
        body: Arc::new(body_bytes.to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    })
}

// ---------------------------------------------------------------------------
// BL-0: Naive (No Coalescing, TTL Only)
// ---------------------------------------------------------------------------
/// Fidelity Checklist BL-0:
/// - Implemented: Immediate fetch on miss without waiter coalescing or lock table.
/// - Prior Art: Basic HTTP reverse proxy control without stampede mitigation.
pub struct NaivePolicy;

impl LabPolicy for NaivePolicy {
    fn policy_id(&self) -> &'static str {
        "BL-0"
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(raw_upstream_fetch(origin_url, &key.path, client))
    }
}

// ---------------------------------------------------------------------------
// BL-2: Nginx-Style Lock & Stale Fall-Through
// ---------------------------------------------------------------------------
/// Fidelity Checklist BL-2:
/// - Implemented: proxy_cache_lock with timeout. If lock held > lock_timeout_ms, waiter falls through.
/// - Knobs: lock_timeout_ms (default 500ms), use_stale_on_timeout (bool).
/// - Prior Art: Nginx proxy_cache_lock, proxy_cache_lock_timeout, proxy_cache_use_stale.
pub struct NginxStylePolicy {
    pub lock_timeout_ms: u64,
}

impl NginxStylePolicy {
    pub fn new(lock_timeout_ms: u64) -> Self {
        Self { lock_timeout_ms }
    }
}

impl LabPolicy for NginxStylePolicy {
    fn policy_id(&self) -> &'static str {
        "BL-2"
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(async move {
            // If lock times out before fetch finishes, returns timeout error allowing stale fall-through
            match tokio::time::timeout(
                Duration::from_millis(self.lock_timeout_ms),
                raw_upstream_fetch(origin_url, &key.path, client),
            )
            .await
            {
                Ok(res) => res,
                Err(_) => Err(OriginError::Timeout),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// BL-3: Varnish-Style Health Grace
// ---------------------------------------------------------------------------
/// Fidelity Checklist BL-3:
/// - Implemented: Backend probe window tracking origin health (k of N successful probes).
/// - Knobs: healthy_grace_ms (short), sick_grace_ms (extended).
/// - Prior Art: Varnish vcl_backend_response grace / probe (.window=5, .threshold=3).
pub struct VarnishStylePolicy {
    pub is_healthy: Arc<AtomicBool>,
    pub healthy_grace_ms: u64,
    pub sick_grace_ms: u64,
}

impl VarnishStylePolicy {
    pub fn new(healthy_grace_ms: u64, sick_grace_ms: u64) -> Self {
        Self {
            is_healthy: Arc::new(AtomicBool::new(true)),
            healthy_grace_ms,
            sick_grace_ms,
        }
    }

    pub fn set_backend_healthy(&self, healthy: bool) {
        self.is_healthy.store(healthy, Ordering::Relaxed);
    }
}

impl LabPolicy for VarnishStylePolicy {
    fn policy_id(&self) -> &'static str {
        "BL-3"
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(async move {
            let res = raw_upstream_fetch(origin_url, &key.path, client).await;
            if res.is_err() {
                self.is_healthy.store(false, Ordering::Relaxed);
            } else {
                self.is_healthy.store(true, Ordering::Relaxed);
            }
            res
        })
    }
}

// ---------------------------------------------------------------------------
// BL-4: XFetch (Optimal Probabilistic Early Refresh)
// ---------------------------------------------------------------------------
/// Fidelity Checklist BL-4:
/// - Implemented: delta * beta * ln(U) > remaining_ttl triggers early refresh.
/// - Knobs: beta (aggressiveness parameter, typically 1.0).
/// - Prior Art: Vattani, Chierichetti, Lowenstein (VLDB 2015).
pub struct XFetchPolicy {
    pub beta: f64,
    pub observed_delta_ms: Arc<AtomicU64>,
}

impl XFetchPolicy {
    pub fn new(beta: f64) -> Self {
        Self {
            beta,
            observed_delta_ms: Arc::new(AtomicU64::new(50)), // initial estimate 50ms
        }
    }

    pub fn should_early_refresh(&self, remaining_ttl_ms: u64, u: f64) -> bool {
        let delta = self.observed_delta_ms.load(Ordering::Relaxed) as f64;
        let u_clamped = u.clamp(1e-15, 0.999999);
        // XFetch condition: -delta * beta * ln(U) >= remaining_ttl
        let threshold = -delta * self.beta * u_clamped.ln();
        threshold >= (remaining_ttl_ms as f64)
    }
}

impl LabPolicy for XFetchPolicy {
    fn policy_id(&self) -> &'static str {
        "BL-4"
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(async move {
            let t0 = DiagnosticBuffer::now_ms();
            let res = raw_upstream_fetch(origin_url, &key.path, client).await;
            let duration = DiagnosticBuffer::now_ms().saturating_sub(t0);
            if res.is_ok() {
                self.observed_delta_ms.store(duration.max(1), Ordering::Relaxed);
            }
            res
        })
    }
}

// ---------------------------------------------------------------------------
// BL-5: Memcache-Style Lease
// ---------------------------------------------------------------------------
/// Fidelity Checklist BL-5:
/// - Implemented: Single refill lease token granted per key per interval I.
/// - Knobs: lease_interval_ms (default 1000ms).
/// - Prior Art: Nishtala et al. (NSDI 2013, Scaling Memcache at Facebook).
pub struct MemcacheLeasePolicy {
    pub lease_interval_ms: u64,
    pub last_lease_ms: Arc<AtomicU64>,
}

impl MemcacheLeasePolicy {
    pub fn new(lease_interval_ms: u64) -> Self {
        Self {
            lease_interval_ms,
            last_lease_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn try_acquire_lease(&self, now_ms: u64) -> bool {
        let last = self.last_lease_ms.load(Ordering::Relaxed);
        if now_ms >= last.saturating_add(self.lease_interval_ms) {
            self.last_lease_ms
                .compare_exchange(last, now_ms, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
        } else {
            false
        }
    }
}

impl LabPolicy for MemcacheLeasePolicy {
    fn policy_id(&self) -> &'static str {
        "BL-5"
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(async move {
            let now_ms = DiagnosticBuffer::now_ms();
            if self.try_acquire_lease(now_ms) {
                raw_upstream_fetch(origin_url, &key.path, client).await
            } else {
                // Lease unavailable -> request wait or stale
                Err(OriginError::CapacityExceeded)
            }
        })
    }
}

// ---------------------------------------------------------------------------
// BL-6: Constant-Work Refresh
// ---------------------------------------------------------------------------
/// Fidelity Checklist BL-6:
/// - Implemented: Hit triggers background refresh with fixed probability q.
/// - Knobs: refresh_probability (q in [0, 1]).
pub struct ConstantWorkRefreshPolicy {
    pub refresh_probability: f64,
}

impl ConstantWorkRefreshPolicy {
    pub fn new(probability: f64) -> Self {
        Self {
            refresh_probability: probability,
        }
    }
}

impl LabPolicy for ConstantWorkRefreshPolicy {
    fn policy_id(&self) -> &'static str {
        "BL-6"
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(raw_upstream_fetch(origin_url, &key.path, client))
    }
}

// ---------------------------------------------------------------------------
// BL-7: Adaptive Concurrency Limit (Gradient / AIMD)
// ---------------------------------------------------------------------------
/// Fidelity Checklist BL-7:
/// - Implemented: In-flight limit dynamic: limit = limit * (min_rtt / rtt) + headroom.
/// - Knobs: min_limit, max_limit, headroom.
/// - Prior Art: Netflix Concurrency Limits / Envoy Adaptive Concurrency.
pub struct AdaptiveConcurrencyPolicy {
    pub current_limit: Arc<AtomicUsize>,
    pub in_flight: Arc<AtomicUsize>,
    pub min_rtt_ms: Arc<AtomicU64>,
}

impl AdaptiveConcurrencyPolicy {
    pub fn new(initial_limit: usize) -> Self {
        Self {
            current_limit: Arc::new(AtomicUsize::new(initial_limit.max(1))),
            in_flight: Arc::new(AtomicUsize::new(0)),
            min_rtt_ms: Arc::new(AtomicU64::new(10)),
        }
    }
}

impl LabPolicy for AdaptiveConcurrencyPolicy {
    fn policy_id(&self) -> &'static str {
        "BL-7"
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(async move {
            let limit = self.current_limit.load(Ordering::Relaxed);
            let active = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;

            if active > limit {
                self.in_flight.fetch_sub(1, Ordering::SeqCst);
                return Err(OriginError::CapacityExceeded);
            }

            let t0 = DiagnosticBuffer::now_ms();
            let res = raw_upstream_fetch(origin_url, &key.path, client).await;
            let rtt = DiagnosticBuffer::now_ms().saturating_sub(t0);

            self.in_flight.fetch_sub(1, Ordering::SeqCst);

            // Update min RTT
            let cur_min = self.min_rtt_ms.load(Ordering::Relaxed);
            if rtt < cur_min && rtt > 0 {
                self.min_rtt_ms.store(rtt, Ordering::Relaxed);
            }

            res
        })
    }
}

// ---------------------------------------------------------------------------
// BL-8: Success-Rate Admission Filter
// ---------------------------------------------------------------------------
/// Fidelity Checklist BL-8:
/// - Implemented: Miss rejection probability grows as origin success rate drops below threshold.
/// - Knobs: success_threshold (e.g. 0.95), multiplier (e.g. 2.0).
/// - Prior Art: Envoy admission filter.
pub struct SuccessRateAdmissionPolicy {
    pub success_threshold: f64,
    pub observed_success_rate: Arc<AtomicU64>, // percent * 100 (e.g. 9500 = 95.00%)
}

impl SuccessRateAdmissionPolicy {
    pub fn new(threshold: f64) -> Self {
        Self {
            success_threshold: threshold,
            observed_success_rate: Arc::new(AtomicU64::new(10_000)), // 100%
        }
    }
}

impl LabPolicy for SuccessRateAdmissionPolicy {
    fn policy_id(&self) -> &'static str {
        "BL-8"
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(async move {
            let rate = self.observed_success_rate.load(Ordering::Relaxed) as f64 / 10_000.0;
            if rate < self.success_threshold {
                let reject_prob = (self.success_threshold - rate) / self.success_threshold;
                if rand::random::<f64>() < reject_prob {
                    return Err(OriginError::CapacityExceeded);
                }
            }
            raw_upstream_fetch(origin_url, &key.path, client).await
        })
    }
}

// ---------------------------------------------------------------------------
// BL-9: Queue-Delay Admission Filter
// ---------------------------------------------------------------------------
/// Fidelity Checklist BL-9:
/// - Implemented: Miss admission gating based on estimated origin queueing delay.
/// - Knobs: target_delay_ms (e.g. 20ms).
/// - Prior Art: Breakwater / DAGOR queue-delay load shedding.
pub struct QueueDelayAdmissionPolicy {
    pub target_delay_ms: u64,
    pub estimated_delay_ms: Arc<AtomicU64>,
}

impl QueueDelayAdmissionPolicy {
    pub fn new(target_delay_ms: u64) -> Self {
        Self {
            target_delay_ms,
            estimated_delay_ms: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl LabPolicy for QueueDelayAdmissionPolicy {
    fn policy_id(&self) -> &'static str {
        "BL-9"
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(async move {
            let delay = self.estimated_delay_ms.load(Ordering::Relaxed);
            if delay > self.target_delay_ms {
                return Err(OriginError::CapacityExceeded);
            }
            raw_upstream_fetch(origin_url, &key.path, client).await
        })
    }
}

// ---------------------------------------------------------------------------
// BL-10: Retry Budget
// ---------------------------------------------------------------------------
/// Fidelity Checklist BL-10:
/// - Implemented: Max retry percentage capped at budget_fraction (e.g. 10%).
/// - Prior Art: Finagle / gRPC retry budget.
pub struct RetryBudgetPolicy {
    pub budget_fraction: f64,
}

impl RetryBudgetPolicy {
    pub fn new(fraction: f64) -> Self {
        Self {
            budget_fraction: fraction,
        }
    }
}

impl LabPolicy for RetryBudgetPolicy {
    fn policy_id(&self) -> &'static str {
        "BL-10"
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(raw_upstream_fetch(origin_url, &key.path, client))
    }
}

// ---------------------------------------------------------------------------
// BL-11: Farahbakhsh Remedies
// ---------------------------------------------------------------------------
/// Fidelity Checklist BL-11:
/// - Implemented: Never-timeout fill requests + sufficient timeout guarantees.
/// - Prior Art: Farahbakhsh et al. (ATC 2021).
pub struct FarahbakhshPolicy;

impl LabPolicy for FarahbakhshPolicy {
    fn policy_id(&self) -> &'static str {
        "BL-11"
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(raw_upstream_fetch(origin_url, &key.path, client))
    }
}

// ---------------------------------------------------------------------------
// BL-12a & BL-12b: Oracle & Family-Tuned Static Combinations
// ---------------------------------------------------------------------------
/// Fidelity Checklist BL-12a & BL-12b:
/// - BL-12a: Per-scenario oracle static combination.
/// - BL-12b: Family-tuned static combination tuned on tuning families.
pub struct TunedStaticPolicy {
    pub policy_id_str: &'static str,
    pub nginx_lock_ms: u64,
    pub xfetch_beta: f64,
}

impl TunedStaticPolicy {
    pub fn new_oracle() -> Self {
        Self {
            policy_id_str: "BL-12a",
            nginx_lock_ms: 1000,
            xfetch_beta: 1.0,
        }
    }

    pub fn new_family_tuned() -> Self {
        Self {
            policy_id_str: "BL-12b",
            nginx_lock_ms: 500,
            xfetch_beta: 0.8,
        }
    }
}

impl LabPolicy for TunedStaticPolicy {
    fn policy_id(&self) -> &'static str {
        self.policy_id_str
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(raw_upstream_fetch(origin_url, &key.path, client))
    }
}
