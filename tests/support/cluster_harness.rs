//! Multi-process cluster test harness for PhantomCache (WP-10).
//!
//! Features:
//! - Monotonically versioned mock origin server with append-only access log.
//! - Automated test PKI and cluster membership generator using `rcgen`.
//! - Child process management launching real `phantom-cache-demo` gateway binaries.
//! - Link-fault proxy integration for controlled network fault injection.
//! - Accurate staleness and time-to-consistency (TTC) measurements.
//! - RAII teardown guaranteeing zero orphaned child processes.

#![allow(dead_code)]

use super::fault_proxy::{FaultMode, FaultProxy};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use phantom_cache_demo::security::cluster::{
    compute_cert_fingerprint, ClusterMembersConfigFile, MemberNode,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::process::{Child, Command};

// ---------------------------------------------------------------------------
// Versioned Mock Origin
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct KeyRecord {
    pub version: u64,
    pub value: String,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone)]
pub struct OriginAccessEntry {
    pub key: String,
    pub served_version: u64,
    pub timestamp_ms: u64,
}

#[derive(Default)]
pub struct MockOriginState {
    pub records: Mutex<HashMap<String, KeyRecord>>,
    pub access_log: Mutex<Vec<OriginAccessEntry>>,
    pub call_count: AtomicU64,
}

pub struct VersionedMockOrigin {
    url: String,
    state: Arc<MockOriginState>,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl VersionedMockOrigin {
    pub async fn start() -> Self {
        let state = Arc::new(MockOriginState::default());
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let state_clone = state.clone();
        let app = Router::new()
            .route("/api/v1/cache/*path", get(Self::handler))
            .route("/*path", get(Self::handler))
            .route("/", get(Self::handler))
            .with_state(state_clone);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://127.0.0.1:{}", addr.port());

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        Self {
            url,
            state,
            shutdown_tx,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn total_calls(&self) -> u64 {
        self.state.call_count.load(Ordering::SeqCst)
    }

    pub fn mutate_key(&self, key: &str, new_value: &str) -> u64 {
        let mut recs = self.state.records.lock().unwrap();
        let norm_key = if key.starts_with('/') {
            key.to_string()
        } else {
            format!("/{}", key)
        };
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let entry = recs.entry(norm_key).or_insert(KeyRecord {
            version: 0,
            value: String::new(),
            updated_at_ms: now_ms,
        });

        entry.version += 1;
        entry.value = new_value.to_string();
        entry.updated_at_ms = now_ms;
        entry.version
    }

    pub fn get_current_version(&self, key: &str) -> u64 {
        let recs = self.state.records.lock().unwrap();
        let norm_key = if key.starts_with('/') {
            key.to_string()
        } else {
            format!("/{}", key)
        };
        recs.get(&norm_key).map(|r| r.version).unwrap_or(0)
    }

    async fn handler(
        State(state): State<Arc<MockOriginState>>,
        path: Option<Path<String>>,
        _headers: HeaderMap,
    ) -> Response {
        state.call_count.fetch_add(1, Ordering::SeqCst);
        let path_str = path.map(|p| p.0).unwrap_or_default();
        let norm_key = if path_str.starts_with('/') {
            path_str
        } else {
            format!("/{}", path_str)
        };

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let (version, value) = {
            let mut recs = state.records.lock().unwrap();
            let entry = recs.entry(norm_key.clone()).or_insert_with(|| KeyRecord {
                version: 1,
                value: format!("initial_val_{}", norm_key),
                updated_at_ms: now_ms,
            });
            (entry.version, entry.value.clone())
        };

        state.access_log.lock().unwrap().push(OriginAccessEntry {
            key: norm_key.clone(),
            served_version: version,
            timestamp_ms: now_ms,
        });

        (
            StatusCode::OK,
            [
                ("content-type", "application/json"),
                ("cache-control", "max-age=60"),
            ],
            Json(serde_json::json!({
                "key": norm_key,
                "version": version,
                "value": value,
            })),
        )
            .into_response()
    }
}

// ---------------------------------------------------------------------------
// PKI & Membership Generator
// ---------------------------------------------------------------------------

pub struct GeneratedClusterPki {
    pub temp_dir: PathBuf,
    pub ca_cert_pem: String,
    pub members_file_path: PathBuf,
    pub node_cert_paths: HashMap<u8, PathBuf>,
    pub node_key_paths: HashMap<u8, PathBuf>,
}

pub fn generate_cluster_pki(
    cluster_id: &str,
    nodes: &[u8],
    node_cluster_addrs: &HashMap<u8, SocketAddr>,
) -> GeneratedClusterPki {
    let temp_dir = std::env::temp_dir().join(format!("phantom_pki_{}", rand::random::<u64>()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Generate CA
    let mut ca_params = rcgen::CertificateParams::default();
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let ca_key = rcgen::KeyPair::generate().unwrap();
    let ca_cert = ca_params.self_signed(&ca_key).unwrap();
    let ca_cert_pem = ca_cert.pem();

    let ca_path = temp_dir.join("cluster_ca.pem");
    std::fs::write(&ca_path, &ca_cert_pem).unwrap();

    let mut node_cert_paths = HashMap::new();
    let mut node_key_paths = HashMap::new();
    let mut member_nodes = Vec::new();
    let mut addresses = HashMap::new();

    for &node_id in nodes {
        let san = format!("node-{}.{}.phantom.internal", node_id, cluster_id);
        let key = rcgen::KeyPair::generate().unwrap();
        let params = rcgen::CertificateParams::new(vec![san]).unwrap();
        let cert = params.signed_by(&key, &ca_cert, &ca_key).unwrap();

        let cert_pem = cert.pem();
        let key_pem = key.serialize_pem();
        let cert_der = cert.der().to_vec();
        let fp = compute_cert_fingerprint(&cert_der);

        let cert_path = temp_dir.join(format!("node_{}.crt", node_id));
        let key_path = temp_dir.join(format!("node_{}.key", node_id));

        std::fs::write(&cert_path, &cert_pem).unwrap();
        std::fs::write(&key_path, &key_pem).unwrap();

        node_cert_paths.insert(node_id, cert_path);
        node_key_paths.insert(node_id, key_path);

        member_nodes.push(MemberNode {
            node_id,
            fingerprints: vec![fp],
            active: true,
        });

        if let Some(addr) = node_cluster_addrs.get(&node_id) {
            addresses.insert(node_id, addr.to_string());
        }
    }

    let config_file = ClusterMembersConfigFile {
        cluster_id: cluster_id.to_string(),
        version: 1,
        members: member_nodes,
        addresses,
    };

    let members_file_path = temp_dir.join("members.json");
    let members_json = serde_json::to_string_pretty(&config_file).unwrap();
    std::fs::write(&members_file_path, members_json).unwrap();

    GeneratedClusterPki {
        temp_dir,
        ca_cert_pem,
        members_file_path,
        node_cert_paths,
        node_key_paths,
    }
}

// ---------------------------------------------------------------------------
// Gateway Process Node
// ---------------------------------------------------------------------------

pub struct GatewayNode {
    pub node_id: u8,
    pub http_addr: SocketAddr,
    pub cluster_addr: SocketAddr,
    pub child: Child,
    /// File receiving the gateway's captured stdout+stderr (inside the PKI temp dir).
    pub log_path: PathBuf,
}

/// Start-up options for `ClusterHarness::start_with_options`.
#[derive(Clone, Debug)]
pub struct ClusterOptions {
    /// Route every inter-node link through a fault proxy.
    pub with_proxies: bool,
    /// Environment overrides applied to every gateway AFTER the harness defaults
    /// (used by the diagnostics test to force a start-up failure).
    pub extra_env: Vec<(String, String)>,
    /// Total budget for every gateway to become ready.
    pub startup_timeout: Duration,
    /// After `/health/ready`, also wait for every gateway to report that its inter-node
    /// connection pool is pre-warmed (marker line in its log).
    pub wait_for_prewarm: bool,
}

impl Default for ClusterOptions {
    fn default() -> Self {
        Self {
            with_proxies: false,
            extra_env: Vec::new(),
            startup_timeout: Duration::from_secs(12),
            wait_for_prewarm: true,
        }
    }
}

/// Marker line printed by a gateway once it holds a pooled connection to every active peer.
pub const PREWARM_MARKER: &str = "[cluster] pool pre-warm complete";

/// Last `max_lines` lines (at most the last 8 KiB) of a gateway log file, for diagnostics.
pub fn tail_of_file(path: &std::path::Path, max_lines: usize) -> String {
    match std::fs::read(path) {
        Ok(mut bytes) => {
            if bytes.len() > 8192 {
                bytes.drain(..bytes.len() - 8192);
            }
            let text = String::from_utf8_lossy(&bytes).to_string();
            let lines: Vec<&str> = text.lines().collect();
            let from = lines.len().saturating_sub(max_lines);
            let out = lines[from..].join("\n");
            if out.is_empty() {
                "<log empty>".to_string()
            } else {
                out
            }
        }
        Err(e) => format!("<log unreadable: {}>", e),
    }
}

fn log_contains(path: &std::path::Path, needle: &str) -> bool {
    std::fs::read(path)
        .map(|b| String::from_utf8_lossy(&b).contains(needle))
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Multi-Process Cluster Harness
// ---------------------------------------------------------------------------

pub struct ClusterHarness {
    pub origin: Arc<VersionedMockOrigin>,
    pub pki: GeneratedClusterPki,
    pub gateways: HashMap<u8, GatewayNode>,
    pub proxies: HashMap<(u8, u8), Arc<FaultProxy>>,
    client: reqwest::Client,
    /// Wall time from spawn until every gateway was ready (and pre-warmed, if awaited).
    pub startup_elapsed: Duration,
}

/// Result of `ClusterHarness::wait_until_invalidated`.
#[derive(Debug, Clone)]
pub struct InvalidationObservation {
    pub elapsed: Duration,
    pub stale_hit_polls: u32,
    pub version_after: Option<u64>,
    pub status_after: Option<String>,
}

impl ClusterHarness {
    /// Launches a cluster of N real gateway processes connected to the versioned origin.
    /// If `with_proxies` is true, an inter-node fault proxy is set up for each pair.
    pub async fn start(node_ids: &[u8], with_proxies: bool) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::start_with_options(
            node_ids,
            ClusterOptions {
                with_proxies,
                ..ClusterOptions::default()
            },
        )
        .await
    }

    /// Launches `node_ids.len()` gateway processes (any number >= 1; ids are arbitrary non-zero
    /// `u8` values, e.g. `[1, 2, 3, 4, 5]`; ports, certificates, membership entries and
    /// proxy pairs are generated per id). On failure every spawned child is killed and the
    /// error carries elapsed time, per-node status, and the failing gateway's captured log tail.
    pub async fn start_with_options(
        node_ids: &[u8],
        opts: ClusterOptions,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Ports are pre-allocated by bind-then-drop, so another process (a concurrently running
        // test binary, or another test in this binary) can take one before the gateway binds it.
        // That race shows up as a gateway exiting with "address in use"; it is retried with fresh
        // ports. Every other failure is returned immediately with full diagnostics.
        const MAX_ATTEMPTS: usize = 4;
        let mut last_err: Option<Box<dyn std::error::Error + Send + Sync>> = None;
        for attempt in 1..=MAX_ATTEMPTS {
            match Self::start_once(node_ids, opts.clone()).await {
                Ok(h) => return Ok(h),
                Err(e) => {
                    let msg = e.to_string();
                    let port_race = msg.contains("os error 10048")
                        || msg.contains("os error 98")
                        || msg.contains("Address already in use")
                        || msg.contains("Only one usage of each socket address")
                        // The mock origin's ephemeral port can land on a port the gateway's upstream policy
                        // unconditionally blocks (e.g. 2049): a fresh origin gets a fresh port.
                        || msg.contains("Sensitive port")
                        || msg.contains("is disallowed by security policy");
                    if port_race && attempt < MAX_ATTEMPTS {
                        eprintln!("[harness] port conflict on start (attempt {attempt}/{MAX_ATTEMPTS}); retrying with fresh ports");
                        last_err = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(last_err.expect("at least one attempt was made"))
    }

    async fn start_once(
        node_ids: &[u8],
        opts: ClusterOptions,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let with_proxies = opts.with_proxies;
        let origin = Arc::new(VersionedMockOrigin::start().await);
        let cluster_id = "phantom-test-cluster";

        // Pre-allocate ephemeral loopback ports for HTTP and cluster listeners
        let mut http_addrs = HashMap::new();
        let mut actual_cluster_addrs = HashMap::new();
        let mut proxies = HashMap::new();

        for &id in node_ids {
            let l_http = TcpListener::bind("127.0.0.1:0").await?;
            http_addrs.insert(id, l_http.local_addr()?);
            drop(l_http);

            let l_cluster = TcpListener::bind("127.0.0.1:0").await?;
            actual_cluster_addrs.insert(id, l_cluster.local_addr()?);
            drop(l_cluster);
        }

        if with_proxies {
            for &from_id in node_ids {
                for &to_id in node_ids {
                    if from_id != to_id {
                        let target = actual_cluster_addrs[&to_id];
                        let proxy = Arc::new(FaultProxy::start(target).await?);
                        proxies.insert((from_id, to_id), proxy);
                    }
                }
            }
        }
        let advertised_cluster_addrs = actual_cluster_addrs.clone();

        let pki = generate_cluster_pki(cluster_id, node_ids, &advertised_cluster_addrs);

        // Find binary executable
        let bin_path = if let Ok(path) = std::env::var("CARGO_BIN_EXE_phantom-cache-demo") {
            PathBuf::from(path)
        } else {
            let mut p = std::env::current_dir()?;
            p.push("target");
            p.push("debug");
            p.push(if cfg!(windows) { "phantom-cache-demo.exe" } else { "phantom-cache-demo" });
            if !p.exists() {
                return Err(format!("Binary not found at {:?}. Run `cargo build` first.", p).into());
            }
            p
        };

        let mut gateways = HashMap::new();

        for &id in node_ids {
            let http_addr = http_addrs[&id];
            let cluster_addr = actual_cluster_addrs[&id];

            // Build peer addrs string: id=addr,id2=addr2...
            let mut peer_strings = Vec::new();
            for &other_id in node_ids {
                if other_id != id {
                    let peer_addr = if with_proxies {
                        proxies[&(id, other_id)].listen_addr()
                    } else {
                        advertised_cluster_addrs[&other_id]
                    };
                    peer_strings.push(format!("{}={}", other_id, peer_addr));
                }
            }
            let peer_addrs_str = peer_strings.join(",");

            let log_path = pki.temp_dir.join(format!("gateway_{}.log", id));
            let log_out = std::fs::File::create(&log_path)?;
            let log_err = log_out.try_clone()?;

            let mut cmd = Command::new(&bin_path);
            cmd.kill_on_drop(true)
                .env("PHANTOM_NODE_ID", id.to_string())
                .env("PHANTOM_BIND_ADDR", http_addr.to_string())
                .env("PHANTOM_CLUSTER_LISTEN_ADDR", cluster_addr.to_string())
                .env("PHANTOM_CLUSTER_PEER_ADDRS", peer_addrs_str)
                .env("PHANTOM_CLUSTER_MEMBERS_FILE", &pki.members_file_path)
                .env("PHANTOM_CLUSTER_CA", pki.temp_dir.join("cluster_ca.pem"))
                .env("PHANTOM_NODE_CERT", &pki.node_cert_paths[&id])
                .env("PHANTOM_NODE_KEY", &pki.node_key_paths[&id])
                .env("PHANTOM_CLUSTER_ID", cluster_id)
                .env("ORIGIN_URL", origin.url())
                .env("PHANTOM_ALLOW_INSECURE_UPSTREAM", "true")
                .env("PHANTOM_MAX_CACHE_BYTES", "67108864") // 64 MB
                .stdout(std::process::Stdio::from(log_out))
                .stderr(std::process::Stdio::from(log_err));
            for (k, v) in &opts.extra_env {
                cmd.env(k, v);
            }

            let child = cmd.spawn()?;
            gateways.insert(
                id,
                GatewayNode {
                    node_id: id,
                    http_addr,
                    cluster_addr,
                    child,
                    log_path,
                },
            );
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Build the harness first so that ANY failure below tears the cluster down (Drop kills
        // children and removes the temp dir) after the diagnostics have been read.
        let mut harness = Self {
            origin,
            pki,
            gateways,
            proxies,
            client,
            startup_elapsed: Duration::ZERO,
        };

        // Wait for every gateway's /health/ready (config valid + ingress accepting connections).
        let start = Instant::now();
        let budget = opts.startup_timeout;
        let mut ready_nodes: Vec<u8> = Vec::new();
        for &id in node_ids {
            let ready_url = format!("http://{}/health/ready", http_addrs[&id]);
            let mut ready = false;
            let mut exited: Option<String> = None;
            while start.elapsed() < budget {
                if let Some(gw) = harness.gateways.get_mut(&id) {
                    if let Ok(Some(status)) = gw.child.try_wait() {
                        exited = Some(format!("{}", status));
                        break;
                    }
                }
                if let Ok(resp) = harness.client.get(&ready_url).send().await {
                    if resp.status() == StatusCode::OK {
                        ready = true;
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            if !ready {
                let msg = harness.startup_failure_message(
                    id,
                    "failed to become ready (/health/ready)",
                    exited.as_deref(),
                    start.elapsed(),
                    budget,
                    &ready_nodes,
                );
                return Err(msg.into());
            }
            ready_nodes.push(id);
        }

        // Cluster pool pre-warm: every gateway announces when it holds a pooled connection to
        // every active peer. Single-node clusters have no peers and never print it.
        if opts.wait_for_prewarm && node_ids.len() > 1 {
            for &id in node_ids {
                let mut warm = false;
                let mut exited: Option<String> = None;
                while start.elapsed() < budget {
                    if let Some(gw) = harness.gateways.get_mut(&id) {
                        if let Ok(Some(status)) = gw.child.try_wait() {
                            exited = Some(format!("{}", status));
                            break;
                        }
                    }
                    // Readiness is polled from the gateway itself (PC-AUTO-047): the gateway's own metrics report
                    // every member Active, which means the cluster listener answers and the member list is live.
                    // It no longer depends on a log line that the product does not print.
                    if Self::all_members_active(&harness.client, harness.gateways[&id].http_addr, node_ids.len()).await {
                        warm = true;
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                if !warm {
                    let msg = harness.startup_failure_message(
                        id,
                        "ready, but its metrics did not report every cluster member Active",
                        exited.as_deref(),
                        start.elapsed(),
                        budget,
                        &ready_nodes,
                    );
                    return Err(msg.into());
                }
            }
        }

        harness.startup_elapsed = start.elapsed();
        Ok(harness)
    }

    /// True when gateway `http_addr` reports itself and `members - 1` peers, all Active, in `metrics.cluster`.
    async fn all_members_active(client: &reqwest::Client, http_addr: SocketAddr, members: usize) -> bool {
        let url = format!("http://{}/api/v1/metrics", http_addr);
        let body = match client.get(&url).send().await {
            Ok(r) => match r.text().await {
                Ok(b) => b,
                Err(_) => return false,
            },
            Err(_) => return false,
        };
        let v: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let c = &v["metrics"]["cluster"];
        let local_ok = c["local"]["state"]["v"].as_str() == Some("Active");
        let peers = match c["peers"].as_array() {
            Some(p) => p,
            None => return false,
        };
        local_ok && peers.len() + 1 == members && peers.iter().all(|p| p["state"]["v"].as_str() == Some("Active"))
    }

    /// Builds the diagnostic message for a failed start-up (node, reason, elapsed time vs budget,
    /// which nodes were already ready, child exit status if it died, and the gateway log tail).
    fn startup_failure_message(
        &self,
        node_id: u8,
        reason: &str,
        exited: Option<&str>,
        elapsed: Duration,
        budget: Duration,
        ready_nodes: &[u8],
    ) -> String {
        let log = self
            .gateways
            .get(&node_id)
            .map(|g| tail_of_file(&g.log_path, 40))
            .unwrap_or_else(|| "<no such gateway>".to_string());
        format!(
            "Gateway node {} {} after {:.2?} (budget {:.2?}); gateway process {}; nodes ready before failure: {:?}\n--- gateway {} captured stdout/stderr (tail) ---\n{}\n--- end gateway log ---",
            node_id,
            reason,
            elapsed,
            budget,
            match exited {
                Some(s) => format!("EXITED with {}", s),
                None => "still running".to_string(),
            },
            ready_nodes,
            node_id,
            log
        )
    }

    /// Tail (last `max_lines` lines) of a gateway's captured stdout/stderr.
    pub fn gateway_log_tail(&self, node_id: u8, max_lines: usize) -> String {
        self.gateways
            .get(&node_id)
            .map(|g| tail_of_file(&g.log_path, max_lines))
            .unwrap_or_else(|| "<no such gateway>".to_string())
    }

    /// Polls `GET key` on `node_id` until the answer is a `200` that is not `X-Cache-Status: HIT`,
    /// or `cap` elapses (non-200 answers, e.g. 5xx, never satisfy it; callers must additionally
    /// assert `version_after` so a 200 with unexpected content cannot pass). Returns the elapsed
    /// time from the call until that answer was received, the number of stale HIT answers seen,
    /// and the version in that answer. NOTE: the elapsed time is dominated by the latency of the
    /// MISS itself (an origin fetch through the gateway), not by propagation alone: it is an upper
    /// bound on propagation, not a propagation measurement. Polling refills
    /// that gateway's cache for `key`. Use ONLY with keys that are warmed on `node_id`.
    pub async fn wait_until_invalidated(
        &self,
        node_id: u8,
        key: &str,
        cap: Duration,
    ) -> Result<InvalidationObservation, String> {
        let t0 = Instant::now();
        let mut stale_hits = 0u32;
        loop {
            if t0.elapsed() > cap {
                return Err(format!(
                    "gateway {} still answered HIT for {} after {:?} ({} stale HIT polls)",
                    node_id, key, cap, stale_hits
                ));
            }
            match self.http_get(node_id, key).await {
                Ok((s, _body, cache, ver)) => {
                    if s == StatusCode::OK && cache.as_deref() == Some("HIT") {
                        stale_hits += 1;
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        continue;
                    }
                    if s != StatusCode::OK {
                        // A 5xx (or any non-200) is NOT evidence of invalidation: keep polling.
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        continue;
                    }
                    return Ok(InvalidationObservation {
                        elapsed: t0.elapsed(),
                        stale_hit_polls: stale_hits,
                        version_after: ver,
                        status_after: cache,
                    });
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(1)).await,
            }
        }
    }

    /// Node states as reported by gateway `via` (`GET /api/v1/metrics`, `nodes[]`; labelled
    /// `inferred` by the gateway: this is the router's own view, not a probe).
    pub async fn node_states(&self, via: u8) -> Result<HashMap<u8, String>, String> {
        let gw = self.gateways.get(&via).ok_or("no such gateway")?;
        let url = format!("http://{}/api/v1/metrics", gw.http_addr);
        let body = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| format!("metrics not JSON: {e}"))?;
        let nodes = v.get("nodes").and_then(|n| n.as_array()).ok_or("metrics has no nodes[]")?;
        let mut out = HashMap::new();
        for n in nodes {
            if let (Some(id), Some(st)) = (n.get("node_id").and_then(|x| x.as_u64()), n.get("state").and_then(|x| x.as_str())) {
                out.insert(id as u8, st.to_string());
            }
        }
        Ok(out)
    }

    /// Polls `node_states(via)` until node `target` reports `want`, or `cap` elapses.
    pub async fn wait_node_state(&self, via: u8, target: u8, want: &str, cap: Duration) -> Result<Duration, String> {
        let t0 = Instant::now();
        let mut last = String::new();
        while t0.elapsed() < cap {
            match self.node_states(via).await {
                Ok(m) => {
                    last = format!("{:?}", m);
                    if m.get(&target).map(|s| s.as_str()) == Some(want) {
                        return Ok(t0.elapsed());
                    }
                }
                Err(e) => last = e,
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        Err(format!("node {target} did not report {want} via gateway {via} within {cap:?} (last view: {last})"))
    }

    /// Explicit readiness step for the invalidation path: one throwaway DELETE per source
    /// gateway, so the DELETE code path, the primary leg and the replica broadcaster have each
    /// been exercised once before any timing-sensitive assertion. Keys are unique per call.
    pub async fn warm_invalidation_paths(&self) -> Result<(), String> {
        let ids: Vec<u8> = self.gateways.keys().copied().collect();
        for src in ids {
            let key = format!("/__warmup_{}_{}", src, rand::random::<u32>());
            let st = self
                .http_delete(src, &key)
                .await
                .map_err(|e| format!("warm-up DELETE on gateway {} failed: {}", src, e))?;
            if st != StatusCode::NO_CONTENT {
                return Err(format!("warm-up DELETE on gateway {} returned {}", src, st));
            }
        }
        Ok(())
    }

    pub fn origin(&self) -> &VersionedMockOrigin {
        &self.origin
    }

    pub fn proxy(&self, from: u8, to: u8) -> Option<&FaultProxy> {
        self.proxies.get(&(from, to)).map(|p| p.as_ref())
    }

    pub async fn set_link_mode(&self, from: u8, to: u8, mode: FaultMode) {
        if let Some(p) = self.proxies.get(&(from, to)) {
            p.set_mode(mode).await;
        }
    }

    pub async fn set_symmetric_partition(&self, node_a: u8, node_b: u8) {
        self.set_link_mode(node_a, node_b, FaultMode::Partition).await;
        self.set_link_mode(node_b, node_a, FaultMode::Partition).await;
    }

    pub async fn heal_partition(&self, node_a: u8, node_b: u8) {
        self.set_link_mode(node_a, node_b, FaultMode::Normal).await;
        self.set_link_mode(node_b, node_a, FaultMode::Normal).await;
    }

    /// Issues an HTTP GET to gateway `node_id`.
    /// Returns `(status, body, x-cache-status header, version from json if present)`.
    pub async fn http_get(
        &self,
        node_id: u8,
        path: &str,
    ) -> Result<(StatusCode, String, Option<String>, Option<u64>), reqwest::Error> {
        let gw = self.gateways.get(&node_id).expect("Invalid node_id");
        let norm_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        let url = format!("http://{}/api/v1/cache{}", gw.http_addr, norm_path);

        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        let cache_status = resp
            .headers()
            .get("x-cache-status")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let body = resp.text().await.unwrap_or_default();
        let version = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("version").and_then(|ver| ver.as_u64()));

        Ok((status, body, cache_status, version))
    }

    /// Issues an HTTP DELETE to gateway `node_id`.
    pub async fn http_delete(
        &self,
        node_id: u8,
        path: &str,
    ) -> Result<StatusCode, reqwest::Error> {
        let gw = self.gateways.get(&node_id).expect("Invalid node_id");
        let norm_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        let url = format!("http://{}/api/v1/cache{}", gw.http_addr, norm_path);

        let resp = self.client.delete(&url).send().await?;
        Ok(resp.status())
    }

    /// Gracefully stops all child processes and cleans up temporary directories.
    pub fn shutdown(&mut self) {
        for gw in self.gateways.values_mut() {
            let _ = gw.child.start_kill();
        }
        for p in self.proxies.values() {
            p.stop();
        }
        // Reap the children (bounded) so open log handles are released before the directory
        // holding the PKI material and gateway logs is removed.
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            let all_exited = self
                .gateways
                .values_mut()
                .all(|gw| matches!(gw.child.try_wait(), Ok(Some(_)) | Err(_)));
            if all_exited {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = std::fs::remove_dir_all(&self.pki.temp_dir);
    }
}

impl Drop for ClusterHarness {
    fn drop(&mut self) {
        self.shutdown();
    }
}
