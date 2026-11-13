pub mod sim {
    pub mod backend;
    pub mod clock;
    pub mod network;
    pub mod rng;
    pub mod scheduler;
    pub mod traffic;
}

pub mod core {
    pub mod cache;
    pub mod coalesce;
    pub mod eviction;
    pub mod freshness;
}

pub mod intel {
    pub mod diagnostics;
    pub mod origin_metrics;
    pub mod sketch;
    pub mod telemetry;
    pub mod workload;
}

pub mod control {
    pub mod pie;
}

pub mod dist {
    pub mod anti_entropy;
    pub mod frame;
    pub mod node;
    pub mod pool;
    pub mod router;
    pub mod session;
}

pub mod api {
    pub mod server;
}

pub mod security;

pub mod intelligence;
pub mod intelligence_v2;

// M7: centralized configuration and readiness state. Registered here because this binary
// compiles its own module tree (the library registers the same files in src/lib.rs).
#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod health;
// G2: the Lab HTTP API (src/api/lab_api.rs) drives the library experiment runners, so this binary
// compiles the same modules. They are used only when PHANTOM_LAB_ENABLED=true.
mod lab;
#[allow(dead_code)]
mod lab_live;

use intelligence::PolicyExecutor;

use api::server::{ApiState, NodeSnapshot, SimulationSnapshot};
use core::cache::PartitionedCache;
use core::coalesce::Coalescer;
use dist::node::{NodeState, VirtualNode};
use sim::backend::BackendDatabase;
use sim::clock::{Clock, VirtualClock};
use sim::network::NetworkLink;
use sim::rng::SeededRngSource;
use sim::traffic::TrafficGenerator;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// Exit codes (documented in docs/user/operate.md):
/// 0 clean shutdown or valid `--check-config`; 1 runtime failure; 2 invalid configuration or usage.
const EXIT_OK: i32 = 0;
const EXIT_RUNTIME: i32 = 1;
const EXIT_CONFIG: i32 = 2;

struct Fatal(i32, String);

fn main() {
    let code = real_main();
    std::process::exit(code);
}

fn real_main() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match config::parse_cli(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[CONFIG ERROR] {}", e);
            eprintln!("{}", config::USAGE);
            return EXIT_CONFIG;
        }
    };
    if cli.help {
        println!("{}", config::USAGE);
        return EXIT_OK;
    }
    if cli.version {
        println!(
            "phantom-cache-demo {} (commit {}, built {})",
            config::VERSION,
            config::BUILD_COMMIT,
            config::BUILD_DATE
        );
        return EXIT_OK;
    }

    let env = config::env_snapshot();
    let config_path = cli
        .config
        .clone()
        .or_else(|| env.get("PHANTOM_CONFIG").filter(|v| !v.is_empty()).cloned());
    let file = match &config_path {
        Some(p) => match std::fs::read_to_string(p) {
            Ok(text) => match config::parse_config_file(&text) {
                Ok(entries) => entries,
                Err(errs) => {
                    for e in errs {
                        eprintln!("[CONFIG ERROR] {} ({})", e, p);
                    }
                    return EXIT_CONFIG;
                }
            },
            Err(e) => {
                eprintln!("[CONFIG ERROR] cannot read configuration file '{}': {}", p, e);
                return EXIT_CONFIG;
            }
        },
        None => Vec::new(),
    };

    let inputs = config::Inputs { env, file, cli: cli.overrides() };
    let mut cfg = match config::resolve(&inputs) {
        Ok(c) => c,
        Err(errs) => {
            for e in errs {
                eprintln!("[CONFIG ERROR] {}", e);
            }
            eprintln!("Startup aborted (fail closed). Run with --help or see docs/user/configuration.md.");
            return EXIT_CONFIG;
        }
    };
    cfg.config_file = config_path;

    let (pre_errors, pre_warnings) = config::preflight(&cfg);
    if !pre_errors.is_empty() {
        for e in pre_errors {
            eprintln!("[CONFIG ERROR] {}", e);
        }
        return EXIT_CONFIG;
    }

    println!("PHANTOM CORE INITIALIZING...");
    print!("{}", cfg.report());
    for w in pre_warnings {
        println!("  WARNING: {}", w);
    }
    if cli.check_config {
        println!("Configuration is valid.");
        return EXIT_OK;
    }

    // Make CLI/file-sourced values visible to code that reads the process environment.
    // Single-threaded here: the runtime has not been created yet.
    for (k, v) in cfg.env_exports() {
        match v {
            Some(val) => std::env::set_var(k, val),
            None => std::env::remove_var(k),
        }
    }

    let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("[FATAL] cannot create async runtime: {}", e);
            return EXIT_RUNTIME;
        }
    };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rt.block_on(run(cfg))));
    rt.shutdown_timeout(std::time::Duration::from_secs(5));
    match outcome {
        Ok(Ok(code)) => code,
        Ok(Err(Fatal(code, msg))) => {
            eprintln!("[FATAL] {}", msg);
            code
        }
        Err(_) => {
            eprintln!("[FATAL] runtime panic (see message above); exiting with status {}", EXIT_RUNTIME);
            EXIT_RUNTIME
        }
    }
}

async fn run(cfg: config::Config) -> Result<i32, Fatal> {
    let clock = Arc::new(VirtualClock::new(0));
    let rng = Arc::new(SeededRngSource::new(42));
    let _traffic = TrafficGenerator::new(rng.clone());

    let _network = Arc::new(NetworkLink::new(10, 2, 0.001));
    let _db = Arc::new(BackendDatabase::new(15, 100));

    let cache = Arc::new(PartitionedCache::new(8));
    cache.set_byte_budget(cfg.get_u64("PHANTOM_MAX_CACHE_BYTES") as usize);
    cache.set_max_stale_ms(cfg.get_u64("PHANTOM_MAX_STALE_MS"));
    cache.set_sweeper_batch_size(cfg.get_u64("PHANTOM_SWEEPER_BATCH_SIZE") as usize);
    cache.set_sweeper_interval_ms(cfg.get_u64("PHANTOM_SWEEPER_INTERVAL_MS"));
    let _coalescer = Arc::new(Coalescer::new(1000));

    let mut nodes = vec![
        Arc::new(VirtualNode::new(1)),
        Arc::new(VirtualNode::new(2)),
        Arc::new(VirtualNode::new(3)),
    ];
    for n in &nodes {
        n.set_state(NodeState::Active);
    }

    // NOTE: this initial snapshot is SYNTHETIC placeholder data (not measured). It is only
    // used by the simulation endpoints; live telemetry comes from the cache counters.
    let initial_snap = SimulationSnapshot {
        version: "1.0".to_string(),
        timestamp_ms: clock.now_ms(),
        throughput_req_sec: 12000.0,
        p50_latency_ms: 10,
        p95_latency_ms: 12,
        p99_latency_ms: 15,
        db_load_ratio: 0.2,
        avoided_compute_dollars: 0.0,
        nodes: vec![
            NodeSnapshot {
                node_id: 1,
                state: "Active".to_string(),
                cpu_utilization: 28.4,
                memory_used_bytes: 536870912,
            },
            NodeSnapshot {
                node_id: 2,
                state: "Active".to_string(),
                cpu_utilization: 28.4,
                memory_used_bytes: 536870912,
            },
            NodeSnapshot {
                node_id: 3,
                state: "Active".to_string(),
                cpu_utilization: 28.4,
                memory_used_bytes: 536870912,
            },
        ],
    };

    let origin_url = cfg.get("ORIGIN_URL").map(|s| s.to_string());
    let telemetry = Arc::new(intel::telemetry::TelemetryCollector::new(10000));

    let bind_addr = cfg.bind_addr();
    let is_network_bind = !bind_addr.ip().is_loopback();
    let allowed_origins = cfg.allowlist();
    let allow_insecure_cluster = cfg.get_bool("PHANTOM_ALLOW_INSECURE_CLUSTER");

    let cluster_id = cfg.get("PHANTOM_CLUSTER_ID").unwrap_or("phantom-cluster").to_string();

    let cluster_registry = if let Some(members_file) = cfg.get("PHANTOM_CLUSTER_MEMBERS_FILE") {
        let registry = security::load_membership_from_file(members_file, &cluster_id).map_err(|e| {
            Fatal(EXIT_CONFIG, format!("Failed to load cluster membership configuration: {}", e))
        })?;
        println!(
            "Authoritative cluster membership loaded: {} members (epoch {})",
            registry.members.len(),
            registry.version
        );
        Some(registry)
    } else {
        None
    };

    let ca_path = cfg.get("PHANTOM_CLUSTER_CA");
    let cert_path = cfg.get("PHANTOM_NODE_CERT");
    let key_path = cfg.get("PHANTOM_NODE_KEY");

    let router: Option<Arc<dist::router::RendezvousRouter>> =
        if let (Some(ca_path), Some(cert_path), Some(key_path)) = (ca_path, cert_path, key_path) {
            let registry = cluster_registry.ok_or_else(|| {
                Fatal(
                    EXIT_CONFIG,
                    "Cluster TLS configured without PHANTOM_CLUSTER_MEMBERS_FILE. Aborting startup to fail closed."
                        .into(),
                )
            })?;

            let read = |label: &str, p: &str| {
                std::fs::read(p).map_err(|e| Fatal(EXIT_CONFIG, format!("Failed to read {} at {}: {}", label, p, e)))
            };
            let ca_pem = read("Cluster CA", ca_path)?;
            let cert_pem = read("Node Certificate", cert_path)?;
            let key_pem = read("Node Key", key_path)?;

            let node_id = cfg.get_u64("PHANTOM_NODE_ID") as u8;

            let cluster_listen_addr = cfg
                .get("PHANTOM_CLUSTER_LISTEN_ADDR")
                .ok_or_else(|| {
                    Fatal(EXIT_CONFIG, "Clustered mode enabled, but PHANTOM_CLUSTER_LISTEN_ADDR is not set.".into())
                })?
                .to_string();
            let parsed_listen_addr: SocketAddr = cluster_listen_addr.parse().map_err(|e| {
                Fatal(
                    EXIT_CONFIG,
                    format!("Invalid PHANTOM_CLUSTER_LISTEN_ADDR '{}': {}", cluster_listen_addr, e),
                )
            })?;

            let mut peer_addrs: std::collections::HashMap<u8, SocketAddr> = std::collections::HashMap::new();
            if let Some(peer_str) = cfg.get("PHANTOM_CLUSTER_PEER_ADDRS") {
                for pair in peer_str.split(',').filter(|s| !s.is_empty()) {
                    if let Some((id_str, addr_str)) = pair.split_once('=') {
                        if let (Ok(id), Ok(addr)) = (id_str.parse::<u8>(), addr_str.parse::<SocketAddr>()) {
                            peer_addrs.insert(id, addr);
                        }
                    }
                }
            }

            let cluster_tls = Arc::new(
                security::ClusterTlsConfig::from_pem(node_id, &cluster_id, &ca_pem, &cert_pem, &key_pem, registry.clone())
                    .map_err(|e| Fatal(EXIT_CONFIG, format!("Failed to initialize Cluster TLS: {}", e)))?,
            );

            let mut configured_nodes = Vec::new();
            for m_id in registry.members.keys() {
                if *m_id == node_id {
                    let local_node = Arc::new(dist::node::VirtualNode::new_with_cache(node_id, cache.clone()));
                    local_node.set_cluster_tls(cluster_tls.clone());
                    local_node.start_tcp_service(&cluster_listen_addr).await.map_err(|e| {
                        Fatal(
                            EXIT_RUNTIME,
                            format!("Failed to start cluster listener on {}: {}", cluster_listen_addr, e),
                        )
                    })?;
                    configured_nodes.push(local_node);
                } else {
                    let peer_addr = peer_addrs.get(m_id).copied().or_else(|| registry.get_address(*m_id));
                    let peer_addr = match peer_addr {
                        Some(a) => a,
                        None => {
                            return Err(Fatal(
                                EXIT_CONFIG,
                                format!(
                                    "Member node {} has no socket address configured via PHANTOM_CLUSTER_PEER_ADDRS or membership addresses map.",
                                    m_id
                                ),
                            ))
                        }
                    };
                    if peer_addr == parsed_listen_addr {
                        return Err(Fatal(
                            EXIT_CONFIG,
                            format!(
                                "Peer node {} address '{}' conflicts with local cluster listener address.",
                                m_id, peer_addr
                            ),
                        ));
                    }
                    let peer_node = Arc::new(dist::node::VirtualNode::new(*m_id));
                    peer_node.set_addr(peer_addr);
                    peer_node.set_state(dist::node::NodeState::Active);
                    configured_nodes.push(peer_node);
                }
            }
            configured_nodes.sort_by_key(|n| n.node_id);
            nodes = configured_nodes;

            eprintln!("[EXPERIMENTAL] Cluster mode (mTLS): the local cluster node shares this gateway's HTTP serving cache, so invalidation frames received from peers purge it; a local miss asks the primary peer (read-through) before the origin; there is no shared global cache. DELETE purges locally, waits for the primary's ack and fans out to every other active member (re-delivered by the repair worker if a peer cannot be reached); see docs/research/adr/ADR-001-present-architecture-and-direction.md");
            let r = Arc::new(dist::router::RendezvousRouter::new(nodes.clone()).with_cluster_tls(cluster_tls));
            r.start_anti_entropy_worker(std::time::Duration::from_millis(250));
            // Open and handshake one pooled connection per member now, so the first invalidation to a
            // destination does not pay TCP connect + mTLS handshake. Never gates /health/live or readiness.
            r.start_prewarm_worker();
            Some(r)
        } else if cluster_registry.is_some() || allow_insecure_cluster {
            eprintln!("[EXPERIMENTAL] Insecure/partial cluster mode: the router holds the three placeholder nodes 1-3 without cluster listeners or peer addresses, so no invalidation frame is sent or received. DELETE purges only this gateway's HTTP cache locally; the router cannot reach a primary, so DELETE reports a gateway timeout (PrimaryInvalidationTimeout). Configure PHANTOM_CLUSTER_CA, PHANTOM_NODE_CERT, PHANTOM_NODE_KEY and PHANTOM_CLUSTER_MEMBERS_FILE for a working cluster.");
            let r = Arc::new(dist::router::RendezvousRouter::new(nodes.clone()));
            r.start_anti_entropy_worker(std::time::Duration::from_millis(250));
            Some(r)
        } else {
            None
        };

    let tls_config = if let Some(ca_path) = cfg.get("PHANTOM_UPSTREAM_CA_CERT") {
        let ca_pem = std::fs::read(ca_path)
            .map_err(|e| Fatal(EXIT_CONFIG, format!("Failed to read CA certificate at {}: {}", ca_path, e)))?;
        Some(Arc::new(
            security::UpstreamTlsConfig::new_with_ca_pem(&ca_pem)
                .map_err(|e| Fatal(EXIT_CONFIG, format!("Invalid PHANTOM_UPSTREAM_CA_CERT PEM: {}", e)))?,
        ))
    } else {
        Some(Arc::new(security::UpstreamTlsConfig::new_default()))
    };

    let upstream_policy = Arc::new(security::UpstreamPolicy {
        origin_url: origin_url.clone(),
        allowed_origins,
        enforce_tls: cfg.get_bool("PHANTOM_ENFORCE_UPSTREAM_TLS"),
        allow_insecure_upstream: cfg.get_bool("PHANTOM_ALLOW_INSECURE_UPSTREAM"),
        allow_lan_origin: cfg.get_bool("PHANTOM_ALLOW_LAN_ORIGIN"),
        allow_loopback_origin: !is_network_bind,
        forward_client_authorization: cfg.get_bool("PHANTOM_FORWARD_UPSTREAM_AUTH"),
        tls_config,
    });

    let security = cfg.get("PHANTOM_JWT_SECRET").map(|sec_key| {
        let issuer = cfg.get("PHANTOM_JWT_ISSUER").unwrap_or("phantom-cache-auth");
        let audience = cfg.get("PHANTOM_JWT_AUDIENCE").unwrap_or("phantom-cache-api");
        let validator = security::JwtValidator::new(issuer, audience, sec_key.as_bytes());
        let mut sec_cfg = security::SecurityConfig::new_with_jwt(validator).with_upstream(upstream_policy.clone());
        // M2 (round-2 review): when the client Authorization header is NOT forwarded upstream
        // (default), the origin response cannot depend on the credential, so authenticated reads
        // use the normal per-tenant cache path (authentication is still checked on every request;
        // responses are stored only when RFC 9111 section 3.5 allows a shared cache to store
        // them: public / s-maxage / must-revalidate). When the credential IS forwarded, the origin
        // may personalise per user, so every authenticated read stays BYPASS-AUTH.
        sec_cfg.enforce_rfc9111_auth_bypass = upstream_policy.forward_client_authorization;
        Arc::new(sec_cfg)
    });

    let api_state = Arc::new(ApiState {
        cache: Some(cache.clone()),
        origin_url,
        snapshot: Mutex::new(initial_snap),
        nodes,
        workload_type: Mutex::new("Normal".to_string()),
        telemetry: Some(telemetry),
        pie: None,
        security,
        upstream_policy: Some(upstream_policy),
        router,
    });

    // G2: Lab HTTP API, OFF unless PHANTOM_LAB_ENABLED=true (routes do not exist otherwise).
    if cfg.get_bool("PHANTOM_LAB_ENABLED") {
        let root = std::path::PathBuf::from(cfg.get("PHANTOM_LAB_DIR").unwrap_or("lab-results"));
        let gw = cfg
            .get("PHANTOM_LAB_GATEWAY_EXE")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::current_exe().ok());
        let svc = api::server::lab_api::LabService::new(api::server::lab_api::LabConfig::new(root.clone(), gw))
            .map_err(|e| {
                let code = if e.kind() == std::io::ErrorKind::InvalidInput { EXIT_CONFIG } else { EXIT_RUNTIME };
                Fatal(code, format!("cannot enable the Lab API (directory {}): {}", root.display(), e))
            })?;
        match api::server::lab_api::bind_children_to_process_lifetime() {
            Ok(()) => println!("[lab] gateway children are bound to this process lifetime (job object, kill on close)"),
            Err(e) => eprintln!("[lab] WARNING: children are NOT bound to this process lifetime: {}", e),
        }
        api::server::install_lab_service(svc);
        println!("[lab] Lab HTTP API enabled under /api/v1/lab (root {})", root.display());
    }

    // Phantom Intelligence (PIE v2) Configuration (value already validated by config).
    let intel_mode = match cfg.get("PHANTOM_INTELLIGENCE").unwrap_or("off") {
        "observe" => intelligence::IntelligenceMode::Observe,
        "advisory" => intelligence::IntelligenceMode::Advisory,
        "canary" => intelligence::IntelligenceMode::Canary,
        "active" => intelligence::IntelligenceMode::Active,
        _ => intelligence::IntelligenceMode::Off,
    };
    let intel_executor = {
        let exec = intelligence::executor::DefaultPolicyExecutor::new(Some(cache));
        exec.set_mode(intel_mode).ok();
        if cfg.get("PHANTOM_ORIGIN_CAP_MAX").is_some() {
            api::server::set_static_origin_cap(cfg.get_u64("PHANTOM_ORIGIN_CAP_MAX") as usize);
            exec.configure_origin_cap(
                cfg.get_u64("PHANTOM_ORIGIN_CAP_MIN") as usize,
                cfg.get_u64("PHANTOM_ORIGIN_CAP_MAX") as usize,
                cfg.get_u64("PHANTOM_ORIGIN_CAP_P99_CEILING_MS") as u32,
                cfg.get_u64("PHANTOM_ORIGIN_CAP_WINDOW") as usize,
            );
        }
        Arc::new(exec)
    };
    let intel_audit = Arc::new(Mutex::new(intelligence::audit::AuditStore::new(100)));
    ApiState::set_intelligence(intel_executor, intel_audit);

    // PIE v2: shadow-mode Phantom Intelligence. Only Off, Observe and Advisory exist as a type; canary/active (only
    // reachable through the explicit experimental opt-in) leave PIE v2 Off. It reads events and writes advisory
    // records; it has no write path to the cache, to any cap or to any epoch.
    {
        let v2_mode = match intel_mode {
            intelligence::IntelligenceMode::Observe => intelligence_v2::Mode::Observe,
            intelligence::IntelligenceMode::Advisory => intelligence_v2::Mode::Advisory,
            _ => intelligence_v2::Mode::Off,
        };
        let pie_cfg = intelligence_v2::PieConfig { mode: v2_mode, ..Default::default() };
        match intelligence_v2::Pie::new(pie_cfg) {
            Ok(pie) => {
                let pie = Arc::new(pie);
                api::server::install_pie_v2(pie.clone());
                if v2_mode != intelligence_v2::Mode::Off {
                    let ticker = pie.clone();
                    tokio::spawn(async move {
                        let mut iv = tokio::time::interval(std::time::Duration::from_millis(1000));
                        loop {
                            iv.tick().await;
                            let now = api::server::process_clock_ms();
                            // A panic inside is caught by the library: it drops PIE to Off and records a Fault.
                            ticker.tick(now);
                            ticker.watchdog(now);
                        }
                    });
                }
                println!("[intelligence] PIE v2 shadow mode: {} (Off, Observe and Advisory only; nothing is ever applied)", v2_mode.as_str());
            }
            Err(e) => eprintln!("[intelligence] PIE v2 not installed: {:?}", e),
        }
    }

    println!("PHANTOM CORE V1.0 INITIALIZED (API Server Mode)");

    // Readiness bookkeeping for the (deferred) /health/ready endpoint: configuration is valid
    // here; "listening" is set only once a TCP connect to the ingress address succeeds.
    let readiness = health::global();
    readiness.mark_config_valid();
    tokio::spawn(wait_for_listener(probe_addr(bind_addr), readiness.clone()));

    // SEC-M7-03: CORS allowlist from the validated configuration (PHANTOM_CORS_ORIGINS, or the
    // profile default); this is the policy of the router the server actually runs.
    let cors_cfg = api::server::CorsConfig::with_origins(cfg.cors_origins())
        .map_err(|e| Fatal(EXIT_CONFIG, format!("PHANTOM_CORS_ORIGINS: {}", e)))?;
    if cors_cfg.allowed_origins().is_empty() {
        println!("[cors] cross-origin access: none (same-origin only)");
    } else {
        println!("[cors] cross-origin access allowed for: {}", cors_cfg.allowed_origins().join(", "));
    }

    let server = api::server::start_server_with_cors(api_state, cors_cfg);
    tokio::pin!(server);
    tokio::select! {
        _ = &mut server => {
            // start_server only returns if the listener stops; that is a failure, not a shutdown.
            readiness.begin_draining();
            Err(Fatal(EXIT_RUNTIME, "server task exited unexpectedly".into()))
        }
        sig = shutdown_signal() => {
            readiness.begin_draining();
            println!("[shutdown] received {}; stopping the ingress listener and exiting (cache is in-memory, nothing to flush)", sig);
            // Dropping `server` closes the listener. In-flight connection draining requires a
            // graceful-shutdown hook in api::server::start_server (deferred integration item).
            Ok(EXIT_OK)
        }
    }
}

/// Address used to probe our own listener: wildcard binds are probed via loopback.
fn probe_addr(bind: SocketAddr) -> SocketAddr {
    let ip: std::net::IpAddr = if bind.ip().is_unspecified() {
        if bind.is_ipv4() {
            std::net::Ipv4Addr::LOCALHOST.into()
        } else {
            std::net::Ipv6Addr::LOCALHOST.into()
        }
    } else {
        bind.ip()
    };
    SocketAddr::new(ip, bind.port())
}

async fn wait_for_listener(probe: SocketAddr, readiness: Arc<health::Readiness>) {
    for _ in 0..600 {
        let attempt =
            tokio::time::timeout(std::time::Duration::from_millis(200), tokio::net::TcpStream::connect(probe)).await;
        if matches!(attempt, Ok(Ok(_))) {
            readiness.mark_listening();
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    eprintln!(
        "[WARN] ingress listener did not accept connections on {} within 30 s; readiness stays false",
        probe
    );
}

async fn shutdown_signal() -> &'static str {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => "SIGINT",
                    _ = term.recv() => "SIGTERM",
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
                "SIGINT"
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
        "Ctrl-C"
    }
}
