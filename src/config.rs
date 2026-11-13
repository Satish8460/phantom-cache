//! Centralized, validated runtime configuration (M7-01).
//!
//! Design (no new dependencies, hand-rolled parsing):
//! * One schema table (`KEYS`) lists every configuration key with its type, unit/range,
//!   default and security impact. The user-facing reference `docs/user/configuration.md`
//!   is checked against this table by `tests/integration/m7_config_tests.rs`.
//! * Precedence: command line flag > environment variable > `--config` file > default.
//! * The process environment remains the interface to code that still reads it directly
//!   (`src/api/server.rs`, `src/dist/router.rs`, `src/security/cluster.rs`). After
//!   validation, `Config::env_exports` yields the normalized values that `main` exports so
//!   that CLI/file values reach those readers as well.
//! * Validation is fail-closed and aggregates all errors. Secrets are never echoed in
//!   errors or in the effective-configuration report.
//! * Resolution is a pure function of `Inputs` (no process state), so it is unit-testable.

use std::collections::BTreeMap;
use std::fmt;
use std::net::SocketAddr;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Stamped at build time by `packaging/build-release.*` via the `PHANTOM_BUILD_COMMIT`
/// compile-time environment variable. "unknown" for ordinary `cargo build`.
pub const BUILD_COMMIT: &str = match option_env!("PHANTOM_BUILD_COMMIT") {
    Some(v) => v,
    None => "unknown",
};
/// Stamped at build time via `PHANTOM_BUILD_DATE` (UTC, ISO-8601) by the packaging script.
pub const BUILD_DATE: &str = match option_env!("PHANTOM_BUILD_DATE") {
    Some(v) => v,
    None => "unknown",
};

/// Minimum JWT HS256 secret length in bytes (RFC 7518 section 3.2 requires a key at least
/// as long as the hash output, 32 bytes for HS256).
pub const MIN_JWT_SECRET_BYTES: usize = 32;

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Bool,
    Uint { min: u64, max: u64 },
    Str,
    Secret,
    Path,
    Addr,
    Url,
    UrlList,
    CorsOrigins,
    Enum(&'static [&'static str]),
    PeerMap,
    ClusterId,
}

/// How a key reaches the running code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wiring {
    /// Applied by `main.rs` from the validated config.
    Main,
    /// Read by downstream code directly from the process environment; `main` exports the
    /// validated value so CLI/file sources reach it.
    EnvExport,
    /// Validated and reported, but the runtime does not honor it yet (see reason).
    NotWired(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub struct KeySpec {
    pub name: &'static str,
    pub kind: Kind,
    pub default: Option<&'static str>,
    pub unit: &'static str,
    pub secret: bool,
    pub wiring: Wiring,
    pub security_impact: &'static str,
    pub description: &'static str,
}

const BOOL: Kind = Kind::Bool;

macro_rules! key {
    ($name:expr, $kind:expr, $default:expr, $unit:expr, $secret:expr, $wiring:expr, $impact:expr, $desc:expr) => {
        KeySpec {
            name: $name,
            kind: $kind,
            default: $default,
            unit: $unit,
            secret: $secret,
            wiring: $wiring,
            security_impact: $impact,
            description: $desc,
        }
    };
}

/// The complete configuration schema. Order is the order of the effective-config report.
pub const KEYS: &[KeySpec] = &[
    key!("PHANTOM_MODE", Kind::Enum(&["development", "production"]), Some("development"), "enum", false, Wiring::Main,
        "high: `production` enables strict fail-closed rules", "Deployment profile. `production` requires auth, a 32-byte+ JWT secret and forbids simulation and every insecure override."),
    key!("PHANTOM_BIND_ADDR", Kind::Addr, Some("127.0.0.1:8085"), "ip:port", false, Wiring::EnvExport,
        "high: a non-loopback address requires JWT auth and TLS", "Ingress HTTP(S) listen address. Must be an IP literal with port."),
    key!("PHANTOM_TLS_CERT", Kind::Path, None, "file path", false, Wiring::EnvExport,
        "high", "PEM certificate chain for ingress TLS. Must be set together with PHANTOM_TLS_KEY."),
    key!("PHANTOM_TLS_KEY", Kind::Path, None, "file path", false, Wiring::EnvExport,
        "high: private key material file; restrict file permissions", "PEM private key for ingress TLS."),
    key!("PHANTOM_ALLOW_INSECURE_HTTP", BOOL, Some("false"), "bool", false, Wiring::EnvExport,
        "high: plaintext ingress on a network interface; lab use only", "Permit plain HTTP on a non-loopback bind (JWT auth is still mandatory)."),
    key!("PHANTOM_REQUIRE_AUTH", BOOL, Some("false"), "bool", false, Wiring::EnvExport,
        "high", "Fail startup unless PHANTOM_JWT_SECRET is configured."),
    key!("PHANTOM_JWT_SECRET", Kind::Secret, None, "bytes (>= 32 recommended)", true, Wiring::Main,
        "critical: HS256 signing key; never logged or printed", "Shared HS256 secret that enables JWT/RBAC enforcement."),
    key!("PHANTOM_JWT_ISSUER", Kind::Str, Some("phantom-cache-auth"), "string", false, Wiring::Main,
        "medium", "Expected `iss` claim."),
    key!("PHANTOM_JWT_AUDIENCE", Kind::Str, Some("phantom-cache-api"), "string", false, Wiring::Main,
        "medium", "Expected `aud` claim."),
    key!("PHANTOM_MAX_CACHE_BYTES", Kind::Uint { min: 1, max: u64::MAX }, Some("268435456"), "bytes", false, Wiring::Main,
        "medium: bounds memory use", "Logical byte budget of the cache (default 256 MiB)."),
    key!("PHANTOM_DEFAULT_TTL_MS", Kind::Uint { min: 1, max: 31_536_000_000 }, Some("60000"), "ms", false,
        Wiring::NotWired("src/api/server.rs passes a literal 60_000 to the freshness engine; the value is validated and reported only"),
        "low", "Default freshness lifetime when the origin sends no freshness headers."),
    key!("PHANTOM_MAX_STALE_MS", Kind::Uint { min: 1, max: 31_536_000_000 }, Some("86400000"), "ms", false, Wiring::Main,
        "medium: hard cap on stale serving", "Hard staleness cap enforced by the execution layer (default 24 h)."),
    key!("PHANTOM_SWEEPER_BATCH_SIZE", Kind::Uint { min: 1, max: 1024 }, Some("32"), "shards per tick", false, Wiring::Main,
        "low", "Lazy invalidation sweeper per-tick shard inspection limit (design cap 32; above 32 warns)."),
    key!("PHANTOM_SWEEPER_INTERVAL_MS", Kind::Uint { min: 1, max: 3_600_000 }, Some("100"), "ms", false, Wiring::Main,
        "low", "Sweeper tick period."),
    key!("ORIGIN_URL", Kind::Url, None, "http(s) URL", false, Wiring::EnvExport,
        "high: upstream destination; validated against the SSRF policy at startup", "Origin base URL the gateway fetches from on a miss. Not PHANTOM_-prefixed (historical name)."),
    key!("PHANTOM_ORIGIN_ALLOWLIST", Kind::UrlList, None, "comma-separated URLs", false, Wiring::EnvExport,
        "high", "Allowlisted origins. When non-empty the origin must match an entry."),
    key!("PHANTOM_ENFORCE_UPSTREAM_TLS", BOOL, Some("false"), "bool", false, Wiring::EnvExport,
        "high", "Reject http:// origins."),
    key!("PHANTOM_ALLOW_INSECURE_UPSTREAM", BOOL, Some("false"), "bool", false, Wiring::EnvExport,
        "high: plaintext upstream; needs an allowlist on network binds", "Permit http:// non-loopback origins (must be allowlisted)."),
    key!("PHANTOM_ALLOW_LAN_ORIGIN", BOOL, Some("false"), "bool", false, Wiring::EnvExport,
        "high: relaxes the SSRF filter for RFC 1918 / ULA ranges", "Permit private-network origins."),
    key!("PHANTOM_FORWARD_UPSTREAM_AUTH", BOOL, Some("false"), "bool", false, Wiring::EnvExport,
        "high: forwards client Authorization to the origin", "Forward the client Authorization header upstream."),
    key!("PHANTOM_UPSTREAM_CA_CERT", Kind::Path, None, "file path", false, Wiring::EnvExport,
        "medium", "Extra CA bundle (PEM) trusted for upstream TLS in addition to the web PKI roots."),
    key!("PHANTOM_CLUSTER_ID", Kind::ClusterId, Some("phantom-cluster"), "label [a-z0-9-]{1,32}", false, Wiring::EnvExport,
        "medium: must equal the cluster label in node certificate SANs", "Cluster identity label."),
    key!("PHANTOM_CLUSTER_MEMBERS_FILE", Kind::Path, None, "file path", false, Wiring::EnvExport,
        "high: authoritative membership and certificate fingerprints", "JSON membership registry (read once at startup)."),
    key!("PHANTOM_CLUSTER_CA", Kind::Path, None, "file path", false, Wiring::EnvExport,
        "high", "Cluster CA certificate (PEM) for mTLS."),
    key!("PHANTOM_NODE_CERT", Kind::Path, None, "file path", false, Wiring::EnvExport,
        "high", "This node's certificate (PEM) for cluster mTLS."),
    key!("PHANTOM_NODE_KEY", Kind::Path, None, "file path", false, Wiring::EnvExport,
        "high: private key material file", "This node's private key (PEM) for cluster mTLS."),
    key!("PHANTOM_NODE_ID", Kind::Uint { min: 1, max: 255 }, Some("1"), "1..=255", false, Wiring::EnvExport,
        "medium", "Local node id; must exist in the membership registry."),
    key!("PHANTOM_CLUSTER_LISTEN_ADDR", Kind::Addr, None, "ip:port", false, Wiring::EnvExport,
        "high", "Cluster mTLS listener address (required when cluster mTLS is configured)."),
    key!("PHANTOM_CLUSTER_PEER_ADDRS", Kind::PeerMap, None, "id=ip:port,...", false, Wiring::EnvExport,
        "medium", "Peer address map overriding addresses in the membership file."),
    key!("PHANTOM_ALLOW_INSECURE_CLUSTER", BOOL, Some("false"), "bool", false, Wiring::EnvExport,
        "high: plaintext cluster transport; loopback development only", "Permit cluster operation without mTLS on loopback."),
    key!("PHANTOM_STRICT_CLUSTER", BOOL, Some("false"), "bool", false, Wiring::EnvExport,
        "medium", "On loopback, require full mTLS material or PHANTOM_ALLOW_INSECURE_CLUSTER."),
    key!("PHANTOM_CLUSTER_TIMEOUT_MS", Kind::Uint { min: 1, max: 600_000 }, Some("150"), "ms", false, Wiring::EnvExport,
        "low", "Per-attempt cluster invalidation timeout."),
    key!("PHANTOM_INTELLIGENCE", Kind::Enum(&["off", "observe", "advisory", "canary", "active"]), Some("off"), "enum", false, Wiring::Main,
        "medium: `canary`/`active` may change cache policy; benefit is not established", "Phantom Intelligence mode."),
    key!("PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE", BOOL, Some("false"), "bool", false, Wiring::Main,
        "high: allows PHANTOM_INTELLIGENCE=canary/active, which may change cache policy", "Explicit opt-in required for PHANTOM_INTELLIGENCE=canary or active (rejected at startup otherwise). Only the research lab sets it."),
    key!("PHANTOM_ORIGIN_CAP_MAX", Kind::Uint { min: 1, max: 1_000_000 }, None, "concurrent origin fetches", false, Wiring::Main,
        "medium: bounds origin load; the static cap is enforced in every intelligence mode", "Static cap on concurrent origin fetches, enforced in every mode (also the upper bound of the experimental AIMD controller in canary/active). Unset = no cap (origin path uncapped)."),
    key!("PHANTOM_ORIGIN_CAP_MIN", Kind::Uint { min: 1, max: 1_000_000 }, Some("1"), "concurrent origin fetches", false, Wiring::Main,
        "low", "Lower bound of the origin concurrency cap (must be <= PHANTOM_ORIGIN_CAP_MAX)."),
    key!("PHANTOM_ORIGIN_CAP_P99_CEILING_MS", Kind::Uint { min: 1, max: 600_000 }, Some("1000"), "ms", false, Wiring::Main,
        "low", "Origin p99 latency ceiling above which the cap controller backs off."),
    key!("PHANTOM_ORIGIN_CAP_WINDOW", Kind::Uint { min: 1, max: 1_000_000 }, Some("100"), "origin observations", false, Wiring::Main,
        "low", "Number of real origin observations per controller step."),
    key!("PHANTOM_SIMULATION", BOOL, Some("false"), "bool", false, Wiring::EnvExport,
        "medium: enables SYNTHETIC dashboard values", "Run the synthetic traffic simulation loop. Values it produces are simulated, not measured."),
    key!("PHANTOM_CORS_ORIGINS", Kind::CorsOrigins, None, "comma-separated origins (max 16)", false, Wiring::Main,
        "medium: browsers from the listed origins may read responses; NOT authentication. Unset = same-origin only (development profile: http://localhost:3000, http://127.0.0.1:3000)",
        "Exact browser origins (scheme://host[:port], no wildcard/path) allowed cross-origin access. Never allows credentials."),
    key!("PHANTOM_LAB_ENABLED", BOOL, Some("false"), "bool", false, Wiring::Main,
        "high: exposes /api/v1/lab (starts experiments, spawns processes, reads retained results); with JWT it needs Observer (reads) / Operator (start, cancel); without JWT only on a loopback bind",
        "Enable the Lab HTTP API (docs/user/lab_api.md). Off by default; when off, every /api/v1/lab path is a 404."),
    key!("PHANTOM_LAB_DIR", Kind::Str, Some("lab-results"), "directory path", false, Wiring::Main,
        "medium: run output is written under <dir>/runs; retained experiments under <dir> are read (read-only)",
        "Lab root directory (runs, retained evidence). Created if missing. Ignored (with a warning) unless PHANTOM_LAB_ENABLED=true."),
    key!("PHANTOM_LAB_GATEWAY_EXE", Kind::Path, None, "file path", false, Wiring::Main,
        "high: executable spawned (several times, on loopback) by scenario_b runs",
        "Gateway binary used by scenario_b runs. Unset = this executable. A debug binary makes the run EXPLORATORY."),
    key!("PHANTOM_CONFIG", Kind::Path, None, "file path", false, Wiring::Main,
        "low", "Path of a configuration file (same as --config)."),
];

pub fn spec(name: &str) -> Option<&'static KeySpec> {
    KEYS.iter().find(|k| k.name == name)
}

/// Keys that `main` exports into the process environment after validation. Meta keys and
/// keys applied directly by `main` are excluded.
fn is_exported(s: &KeySpec) -> bool {
    s.wiring == Wiring::EnvExport
}

// ----------------------------------------------------------------------------------------
// Errors
// ----------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub key: Option<String>,
    pub message: String,
}

impl ConfigError {
    fn key(key: &str, message: impl Into<String>) -> Self {
        Self { key: Some(key.to_string()), message: message.into() }
    }
    fn general(message: impl Into<String>) -> Self {
        Self { key: None, message: message.into() }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.key {
            Some(k) => write!(f, "{}: {}", k, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for ConfigError {}

// ----------------------------------------------------------------------------------------
// Command line
// ----------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliArgs {
    pub config: Option<String>,
    pub bind: Option<String>,
    pub mode: Option<String>,
    pub origin: Option<String>,
    pub check_config: bool,
    pub help: bool,
    pub version: bool,
}

pub const USAGE: &str = "\
PhantomCache gateway

USAGE:
    phantom-cache-demo [OPTIONS]

OPTIONS:
    --config <file>     Read KEY=VALUE configuration from <file> (same as PHANTOM_CONFIG)
    --bind <ip:port>    Ingress listen address (same as PHANTOM_BIND_ADDR)
    --mode <profile>    development (default) | production (same as PHANTOM_MODE)
    --origin <url>      Origin base URL (same as ORIGIN_URL)
    --check-config      Validate configuration, print the effective configuration, exit
                        (0 = valid, 2 = invalid). Does not start the server.
    -V, --version       Print version and build stamp
    -h, --help          Print this help

Precedence: command line > environment > configuration file > default.
Exit codes: 0 clean shutdown / valid config, 1 runtime failure, 2 invalid configuration or usage.
Reference: docs/user/configuration.md";

pub fn parse_cli(args: &[String]) -> Result<CliArgs, ConfigError> {
    let mut out = CliArgs::default();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        let (flag, inline): (&str, Option<String>) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg, None),
        };
        let mut take_value = |name: &str| -> Result<String, ConfigError> {
            if let Some(v) = inline.clone() {
                return Ok(v);
            }
            i += 1;
            match args.get(i) {
                Some(v) if !v.starts_with("--") => Ok(v.clone()),
                _ => Err(ConfigError::general(format!("option {} requires a value", name))),
            }
        };
        match flag {
            "--config" => out.config = Some(take_value("--config")?),
            "--bind" => out.bind = Some(take_value("--bind")?),
            "--mode" => out.mode = Some(take_value("--mode")?),
            "--origin" => out.origin = Some(take_value("--origin")?),
            "--check-config" => out.check_config = true,
            "--help" | "-h" => out.help = true,
            "--version" | "-V" => out.version = true,
            other => {
                return Err(ConfigError::general(format!(
                    "unknown option '{}' (see --help)",
                    other
                )))
            }
        }
        i += 1;
    }
    Ok(out)
}

impl CliArgs {
    /// CLI flags expressed as (config key, raw value) overrides.
    pub fn overrides(&self) -> Vec<(&'static str, String)> {
        let mut v = Vec::new();
        if let Some(x) = &self.bind {
            v.push(("PHANTOM_BIND_ADDR", x.clone()));
        }
        if let Some(x) = &self.mode {
            v.push(("PHANTOM_MODE", x.clone()));
        }
        if let Some(x) = &self.origin {
            v.push(("ORIGIN_URL", x.clone()));
        }
        if let Some(x) = &self.config {
            v.push(("PHANTOM_CONFIG", x.clone()));
        }
        v
    }
}

// ----------------------------------------------------------------------------------------
// Configuration file: `KEY=VALUE` lines, `#` full-line comments, optional `export ` prefix,
// optional surrounding double quotes. Unknown keys and duplicates are errors.
// ----------------------------------------------------------------------------------------

pub fn parse_config_file(text: &str) -> Result<Vec<(String, String)>, Vec<ConfigError>> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut errors = Vec::new();
    for (idx, raw_line) in text.lines().enumerate() {
        let lineno = idx + 1;
        let line = raw_line.trim();
        let line = line.strip_prefix('\u{feff}').unwrap_or(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
        let Some((k, v)) = line.split_once('=') else {
            errors.push(ConfigError::general(format!(
                "config file line {}: expected KEY=VALUE",
                lineno
            )));
            continue;
        };
        let key = k.trim();
        if spec(key).is_none() {
            errors.push(ConfigError::general(format!(
                "config file line {}: unknown key '{}'",
                lineno, key
            )));
            continue;
        }
        if key == "PHANTOM_CONFIG" {
            errors.push(ConfigError::general(format!(
                "config file line {}: PHANTOM_CONFIG cannot be set inside a configuration file",
                lineno
            )));
            continue;
        }
        let mut val = v.trim();
        if val.len() >= 2 && val.starts_with('"') && val.ends_with('"') {
            val = &val[1..val.len() - 1];
        }
        if out.iter().any(|(ek, _)| ek == key) {
            errors.push(ConfigError::general(format!(
                "config file line {}: duplicate key '{}'",
                lineno, key
            )));
            continue;
        }
        out.push((key.to_string(), val.to_string()));
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

// ----------------------------------------------------------------------------------------
// Resolution
// ----------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Default,
    File,
    Env,
    Cli,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::Default => "default",
            Source::File => "file",
            Source::Env => "env",
            Source::Cli => "cli",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Inputs {
    /// Relevant process environment (`PHANTOM_*` and `ORIGIN_URL`), see `env_snapshot`.
    pub env: BTreeMap<String, String>,
    /// Parsed configuration file entries (already checked by `parse_config_file`).
    pub file: Vec<(String, String)>,
    /// Command line overrides, see `CliArgs::overrides`.
    pub cli: Vec<(&'static str, String)>,
}

pub fn env_snapshot() -> BTreeMap<String, String> {
    std::env::vars()
        .filter(|(k, _)| k.starts_with("PHANTOM_") || k == "ORIGIN_URL")
        .collect()
}

#[derive(Debug, Clone)]
struct Entry {
    value: Option<String>,
    source: Source,
}

#[derive(Clone)]
pub struct Config {
    entries: BTreeMap<&'static str, Entry>,
    pub warnings: Vec<String>,
    pub config_file: Option<String>,
}

fn normalize(spec: &KeySpec, raw: &str) -> Result<String, String> {
    let shown = |s: &str| if spec.secret { "<redacted>".to_string() } else { s.to_string() };
    match spec.kind {
        Kind::Bool => match raw.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Ok("true".into()),
            "false" | "0" => Ok("false".into()),
            _ => Err(format!(
                "invalid boolean '{}' (expected true, false, 1 or 0; anything else is rejected rather than silently read as false)",
                raw
            )),
        },
        Kind::Uint { min, max } => {
            let n: u64 = raw.trim().parse().map_err(|_| {
                format!("invalid integer '{}' (expected an unsigned number, unit: {})", raw, spec.unit)
            })?;
            if n < min || n > max {
                return Err(format!("{} is out of range {}..={} (unit: {})", n, min, max, spec.unit));
            }
            Ok(n.to_string())
        }
        Kind::Str | Kind::Path => {
            if raw.trim().is_empty() {
                Err("must not be blank".into())
            } else {
                Ok(raw.to_string())
            }
        }
        Kind::Secret => {
            if raw.is_empty() {
                Err("must not be empty".into())
            } else {
                Ok(raw.to_string())
            }
        }
        Kind::Addr => raw
            .trim()
            .parse::<SocketAddr>()
            .map(|a| a.to_string())
            .map_err(|_| {
                format!(
                    "invalid socket address '{}' (expected an IP literal with port, e.g. 127.0.0.1:8085 or [::1]:8085)",
                    shown(raw)
                )
            }),
        Kind::Url => check_url(raw.trim()).map(|_| raw.trim().to_string()),
        Kind::UrlList => {
            let mut items = Vec::new();
            for part in raw.split(',') {
                let p = part.trim();
                if p.is_empty() {
                    continue;
                }
                check_url(p)?;
                items.push(p.to_string());
            }
            Ok(items.join(","))
        }
        // An explicitly set but EMPTY value is valid: "no cross-origin access".
        Kind::CorsOrigins if raw.is_empty() => Ok(String::new()),
        Kind::CorsOrigins => parse_cors_origins(raw).map(|v| v.join(",")),
        Kind::Enum(allowed) => {
            let lower = raw.trim().to_ascii_lowercase();
            if allowed.contains(&lower.as_str()) {
                Ok(lower)
            } else {
                Err(format!("invalid value '{}' (expected one of: {})", raw, allowed.join(", ")))
            }
        }
        Kind::PeerMap => {
            let mut items = Vec::new();
            for part in raw.split(',') {
                let p = part.trim();
                if p.is_empty() {
                    continue;
                }
                let (id, addr) = p
                    .split_once('=')
                    .ok_or_else(|| format!("malformed peer mapping '{}' (expected id=ip:port)", p))?;
                let id: u8 = id
                    .trim()
                    .parse()
                    .ok()
                    .filter(|n| *n >= 1)
                    .ok_or_else(|| format!("invalid node id in peer mapping '{}' (expected 1..=255)", p))?;
                let addr: SocketAddr = addr
                    .trim()
                    .parse()
                    .map_err(|_| format!("invalid address in peer mapping '{}'", p))?;
                items.push(format!("{}={}", id, addr));
            }
            Ok(items.join(","))
        }
        Kind::ClusterId => {
            let s = raw.trim();
            let ok = !s.is_empty()
                && s.len() <= 32
                && !s.starts_with('-')
                && !s.ends_with('-')
                && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if ok {
                Ok(s.to_string())
            } else {
                Err(format!(
                    "invalid cluster id '{}' (1..=32 chars of a-z, 0-9, '-'; no leading/trailing '-'; must match the SAN label)",
                    raw
                ))
            }
        }
    }
}

/// Maximum number of entries accepted in `PHANTOM_CORS_ORIGINS`.
pub const MAX_CORS_ORIGINS: usize = 16;

/// Origins the development profile allows when `PHANTOM_CORS_ORIGINS` is not set. These are the
/// Vite dev-server origins (`ui/vite.config.js`: `server.port = 3000`). The production profile has
/// no default allowlist (same-origin only).
pub const DEV_CORS_ORIGINS: [&str; 2] = ["http://localhost:3000", "http://127.0.0.1:3000"];

/// Validate and normalize `PHANTOM_CORS_ORIGINS` (SEC-M7-03). Accepts a comma-separated list of
/// exact origins `scheme://host[:port]`: http/https only; no wildcard, no userinfo, no path
/// (not even a trailing `/`), query or fragment; no empty entries; at most 16 entries. Entries are
/// normalized (lowercase host, default port dropped) so they match a browser `Origin` header
/// byte for byte; duplicates after normalization are removed.
pub fn parse_cors_origins(raw: &str) -> Result<Vec<String>, String> {
    let parts: Vec<&str> = raw.split(',').collect();
    if parts.len() > MAX_CORS_ORIGINS {
        return Err(format!("{} entries given; at most {} are allowed", parts.len(), MAX_CORS_ORIGINS));
    }
    let mut out: Vec<String> = Vec::new();
    for part in parts {
        let p = part.trim();
        if p.is_empty() {
            return Err("empty entry (check for a leading, trailing or doubled comma)".to_string());
        }
        // Never echo userinfo (or anything containing '@') from a rejected entry: it may be a secret.
        let shown: String = if p.contains('@') {
            "<entry containing '@' not shown>".to_string()
        } else {
            p.chars().take(120).collect()
        };
        if p.contains('*') {
            return Err(format!("'{}': wildcards are not allowed; list exact origins", shown));
        }
        if p.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(format!("'{}': whitespace inside an origin is not allowed", shown));
        }
        let u = url::Url::parse(p).map_err(|e| format!("'{}': not a valid origin ({})", shown, e))?;
        if u.scheme() != "http" && u.scheme() != "https" {
            return Err(format!("'{}': scheme must be http or https", shown));
        }
        if u.host_str().is_none() {
            return Err(format!("'{}': missing host", shown));
        }
        if !u.username().is_empty() || u.password().is_some() {
            return Err(format!("'{}': credentials in an origin are not allowed", shown));
        }
        if u.query().is_some() || u.fragment().is_some() || u.path() != "/" || p.ends_with('/') {
            return Err(format!(
                "'{}': an origin is scheme://host[:port] only (no path, trailing slash, query or fragment)",
                shown
            ));
        }
        let norm = u.origin().ascii_serialization();
        if !out.contains(&norm) {
            out.push(norm);
        }
    }
    Ok(out)
}

fn check_url(s: &str) -> Result<(), String> {
    let u = url::Url::parse(s).map_err(|e| format!("invalid URL '{}': {}", s, e))?;
    if u.scheme() != "http" && u.scheme() != "https" {
        return Err(format!("URL '{}' must use http or https", s));
    }
    if u.host_str().is_none() {
        return Err(format!("URL '{}' has no host", s));
    }
    Ok(())
}

/// Resolve configuration from `inputs`. Returns every error found, not just the first.
pub fn resolve(inputs: &Inputs) -> Result<Config, Vec<ConfigError>> {
    let mut errors: Vec<ConfigError> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut entries: BTreeMap<&'static str, Entry> = BTreeMap::new();

    for k in inputs.env.keys() {
        if k.starts_with("PHANTOM_") && spec(k).is_none() {
            warnings.push(format!("unknown environment variable {} is ignored", k));
        }
    }

    for s in KEYS {
        let cli = inputs.cli.iter().find(|(k, _)| *k == s.name).map(|(_, v)| v.as_str());
        let env = inputs.env.get(s.name).map(|v| v.as_str());
        let file = inputs.file.iter().find(|(k, _)| k == s.name).map(|(_, v)| v.as_str());
        // An empty value counts as "not set" at that layer.
        let chosen = [(cli, Source::Cli), (env, Source::Env), (file, Source::File)]
            .into_iter()
            .find(|(v, _)| matches!(v, Some(x) if !x.is_empty() || s.name == "PHANTOM_CORS_ORIGINS"));
        let entry = match chosen {
            Some((Some(raw), source)) => match normalize(s, raw) {
                Ok(v) => Entry { value: Some(v), source },
                Err(msg) => {
                    errors.push(ConfigError::key(s.name, format!("{} [source: {}]", msg, source.as_str())));
                    Entry { value: None, source }
                }
            },
            _ => Entry { value: s.default.map(|d| d.to_string()), source: Source::Default },
        };
        entries.insert(s.name, entry);
    }

    let cfg = Config { entries, warnings, config_file: None };
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut cfg = cfg;
    let mut cross_errors = Vec::new();
    let mut cross_warnings = Vec::new();
    cfg.validate_cross(&mut cross_errors, &mut cross_warnings);
    cfg.warnings.extend(cross_warnings);
    if cross_errors.is_empty() {
        Ok(cfg)
    } else {
        Err(cross_errors)
    }
}

impl Config {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).and_then(|e| e.value.as_deref())
    }

    pub fn source(&self, key: &str) -> Source {
        self.entries.get(key).map(|e| e.source).unwrap_or(Source::Default)
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.get(key) == Some("true")
    }

    pub fn get_u64(&self, key: &str) -> u64 {
        self.get(key).and_then(|v| v.parse().ok()).unwrap_or(0)
    }

    pub fn bind_addr(&self) -> SocketAddr {
        self.get("PHANTOM_BIND_ADDR")
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8085)))
    }

    pub fn is_production(&self) -> bool {
        self.get("PHANTOM_MODE") == Some("production")
    }

    pub fn allowlist(&self) -> Vec<String> {
        self.get("PHANTOM_ORIGIN_ALLOWLIST")
            .map(|v| v.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }

    /// Effective CORS allowlist: the validated `PHANTOM_CORS_ORIGINS`; if unset, the Vite dev
    /// origins in the development profile and nothing (same-origin only) in production.
    pub fn cors_origins(&self) -> Vec<String> {
        match self.get("PHANTOM_CORS_ORIGINS") {
            // set but empty: explicitly no cross-origin access, in every profile
            Some("") => Vec::new(),
            Some(v) => v.split(',').map(|s| s.to_string()).collect(),
            None if self.is_production() => Vec::new(),
            None => DEV_CORS_ORIGINS.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn cluster_enabled(&self) -> bool {
        self.get("PHANTOM_CLUSTER_MEMBERS_FILE").is_some()
            || self.get("PHANTOM_CLUSTER_LISTEN_ADDR").is_some()
            || self.get("PHANTOM_CLUSTER_CA").is_some()
            || self.get("PHANTOM_NODE_CERT").is_some()
            || self.get("PHANTOM_NODE_KEY").is_some()
            || self.get_bool("PHANTOM_ALLOW_INSECURE_CLUSTER")
    }

    /// Environment variables `main` must set so that downstream readers observe the
    /// validated, source-merged value. `None` means "remove the variable".
    pub fn env_exports(&self) -> Vec<(&'static str, Option<String>)> {
        let mut out = Vec::new();
        for s in KEYS.iter().filter(|s| is_exported(s)) {
            let e = &self.entries[s.name];
            if e.source == Source::Default {
                continue; // downstream defaults equal the schema defaults
            }
            if s.name == "PHANTOM_STRICT_CLUSTER" {
                // Downstream checks presence only, so `false` must remove the variable.
                out.push((s.name, if e.value.as_deref() == Some("true") { Some("1".to_string()) } else { None }));
            } else {
                out.push((s.name, e.value.clone()));
            }
        }
        out
    }

    fn validate_cross(&self, errors: &mut Vec<ConfigError>, warnings: &mut Vec<String>) {
        let addr = self.bind_addr();
        let network = !addr.ip().is_loopback();
        let prod = self.is_production();
        let secret = self.get("PHANTOM_JWT_SECRET");
        let require_auth = self.get_bool("PHANTOM_REQUIRE_AUTH");
        let tls_cert = self.get("PHANTOM_TLS_CERT");
        let tls_key = self.get("PHANTOM_TLS_KEY");

        // Ingress TLS pairing.
        if tls_cert.is_some() != tls_key.is_some() {
            errors.push(ConfigError::general(
                "PHANTOM_TLS_CERT and PHANTOM_TLS_KEY must be set together",
            ));
        }

        // JWT secret strength (never prints the secret).
        if let Some(sec) = secret {
            if sec.len() < MIN_JWT_SECRET_BYTES {
                let msg = format!(
                    "secret is {} bytes; at least {} bytes are required for an HS256 key",
                    sec.len(),
                    MIN_JWT_SECRET_BYTES
                );
                if require_auth || network || prod {
                    errors.push(ConfigError::key("PHANTOM_JWT_SECRET", msg));
                } else {
                    warnings.push(format!("PHANTOM_JWT_SECRET {}", msg));
                }
            }
        }

        // Ingress fail-closed rules, shared with the runtime check in server.rs.
        if let Err(e) = crate::security::validate_startup_security(
            &addr,
            secret.is_some(),
            tls_cert.is_some() && tls_key.is_some(),
            require_auth,
            self.get_bool("PHANTOM_ALLOW_INSECURE_HTTP"),
        ) {
            errors.push(ConfigError::general(e.to_string()));
        }

        // Upstream fail-closed rules.
        let allowlist = self.allowlist();
        if let Err(e) = crate::security::validate_upstream_startup_security(
            &addr,
            self.get_bool("PHANTOM_ALLOW_INSECURE_UPSTREAM"),
            !allowlist.is_empty(),
        ) {
            errors.push(ConfigError::general(e.to_string()));
        }

        // The configured origin must pass the same policy every request is checked against;
        // otherwise every miss would fail at runtime.
        if let Some(origin) = self.get("ORIGIN_URL") {
            if let Ok(parsed) = url::Url::parse(origin) {
                let policy = crate::security::UpstreamPolicy {
                    origin_url: Some(origin.to_string()),
                    allowed_origins: allowlist.clone(),
                    enforce_tls: self.get_bool("PHANTOM_ENFORCE_UPSTREAM_TLS"),
                    allow_insecure_upstream: self.get_bool("PHANTOM_ALLOW_INSECURE_UPSTREAM"),
                    allow_lan_origin: self.get_bool("PHANTOM_ALLOW_LAN_ORIGIN"),
                    allow_loopback_origin: !network,
                    forward_client_authorization: self.get_bool("PHANTOM_FORWARD_UPSTREAM_AUTH"),
                    tls_config: None,
                };
                if let Err(e) = policy.validate_target(&parsed) {
                    errors.push(ConfigError::key(
                        "ORIGIN_URL",
                        format!("rejected by the upstream security policy: {}", e),
                    ));
                }
            }
        }

        // Cluster rules apply only when clustering is requested. (A single network-bound node
        // must not need cluster mTLS material.)
        let ca = self.get("PHANTOM_CLUSTER_CA").is_some();
        let cert = self.get("PHANTOM_NODE_CERT").is_some();
        let key = self.get("PHANTOM_NODE_KEY").is_some();
        if self.cluster_enabled() {
            match crate::security::validate_cluster_startup_security(
                &addr,
                ca,
                cert,
                key,
                self.get_bool("PHANTOM_ALLOW_INSECURE_CLUSTER"),
            ) {
                Ok(()) => {}
                // The security function reads the raw process variable PHANTOM_STRICT_CLUSTER by
                // presence; the validated boolean checked below is authoritative.
                Err(crate::security::StartupSecurityError::LoopbackClusterWithoutInsecureOverride) => {}
                Err(e) => errors.push(ConfigError::general(e.to_string())),
            }
            if self.get_bool("PHANTOM_STRICT_CLUSTER")
                && addr.ip().is_loopback()
                && !(ca && cert && key)
                && !self.get_bool("PHANTOM_ALLOW_INSECURE_CLUSTER")
            {
                errors.push(ConfigError::general(
                    "PHANTOM_STRICT_CLUSTER is set but cluster mTLS material (PHANTOM_CLUSTER_CA, PHANTOM_NODE_CERT, PHANTOM_NODE_KEY) is incomplete and PHANTOM_ALLOW_INSECURE_CLUSTER is not set",
                ));
            }
            if ca && cert && key {
                if self.get("PHANTOM_CLUSTER_MEMBERS_FILE").is_none() {
                    errors.push(ConfigError::general(
                        "cluster mTLS is configured but PHANTOM_CLUSTER_MEMBERS_FILE is not set",
                    ));
                }
                if self.get("PHANTOM_CLUSTER_LISTEN_ADDR").is_none() {
                    errors.push(ConfigError::general(
                        "cluster mTLS is configured but PHANTOM_CLUSTER_LISTEN_ADDR is not set",
                    ));
                }
            } else if self.get("PHANTOM_CLUSTER_LISTEN_ADDR").is_some() {
                warnings.push(
                    "PHANTOM_CLUSTER_LISTEN_ADDR is only used when cluster mTLS material is complete".to_string(),
                );
            }
            warnings.push(
                "cluster mode is EXPERIMENTAL: peer invalidations purge this gateway's HTTP cache; a local miss asks the primary peer (read-through) before the origin; there is no shared global cache; see docs/research/adr/ADR-001-present-architecture-and-direction.md".to_string(),
            );
        }

        // Not-wired keys: warn when the operator changed them.
        for s in KEYS {
            if let Wiring::NotWired(reason) = s.wiring {
                if self.source(s.name) != Source::Default && self.get(s.name) != s.default {
                    warnings.push(format!("{} is set but not honored by the runtime: {}", s.name, reason));
                }
            }
        }

        // Experimental intelligence modes need an explicit opt-in (PIE design section 8, D-017).
        if matches!(self.get("PHANTOM_INTELLIGENCE"), Some("canary") | Some("active")) && !self.get_bool("PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE") {
            errors.push(ConfigError::key("PHANTOM_INTELLIGENCE", "canary and active are experimental and rejected unless PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE=true is set; use off, observe or advisory"));
        }

        // Origin concurrency cap.
        let cap_max = self.get("PHANTOM_ORIGIN_CAP_MAX").map(|_| self.get_u64("PHANTOM_ORIGIN_CAP_MAX"));
        match cap_max {
            Some(max) => {
                if self.get_u64("PHANTOM_ORIGIN_CAP_MIN") > max {
                    errors.push(ConfigError::key("PHANTOM_ORIGIN_CAP_MIN", "must be <= PHANTOM_ORIGIN_CAP_MAX"));
                }
                if !matches!(self.get("PHANTOM_INTELLIGENCE"), Some("canary") | Some("active")) {
                    warnings.push("PHANTOM_ORIGIN_CAP_MAX is enforced as a static cap on concurrent origin fetches in every mode; only the adaptive AIMD controller (and so PHANTOM_ORIGIN_CAP_MIN/_P99_CEILING_MS/_WINDOW) is inert outside canary/active".to_string());
                }
            }
            None => {
                for k in ["PHANTOM_ORIGIN_CAP_MIN", "PHANTOM_ORIGIN_CAP_P99_CEILING_MS", "PHANTOM_ORIGIN_CAP_WINDOW"] {
                    if self.source(k) != Source::Default {
                        warnings.push(format!("{} is set but PHANTOM_ORIGIN_CAP_MAX is not; the origin cap is not configured", k));
                    }
                }
            }
        }
        // Lab API (G2): anonymous access is allowed only on a loopback bind.
        if self.get_bool("PHANTOM_LAB_ENABLED") {
            if secret.is_none() && network {
                errors.push(ConfigError::key(
                    "PHANTOM_LAB_ENABLED",
                    "the Lab API without JWT authentication (PHANTOM_JWT_SECRET) requires a loopback PHANTOM_BIND_ADDR",
                ));
            }
            warnings.push("PHANTOM_LAB_ENABLED=true: the Lab API can start experiments and spawn processes; results are SIMULATED (t1_matrix) or MEASURED_LOOPBACK (scenario_b)".to_string());
        } else {
            for k in ["PHANTOM_LAB_DIR", "PHANTOM_LAB_GATEWAY_EXE"] {
                if self.source(k) != Source::Default {
                    warnings.push(format!("{} is set but PHANTOM_LAB_ENABLED is not true; it is ignored", k));
                }
            }
        }
        if self.get_u64("PHANTOM_SWEEPER_BATCH_SIZE") > 32 {
            warnings.push("PHANTOM_SWEEPER_BATCH_SIZE above the design cap of 32 may hold shard locks longer per tick".to_string());
        }
        if self.get_u64("PHANTOM_MAX_CACHE_BYTES") < 1_048_576 {
            warnings.push("PHANTOM_MAX_CACHE_BYTES is below 1 MiB; the cache will hold very few entries".to_string());
        }
        if !prod && network && self.get("PHANTOM_CORS_ORIGINS").is_none() {
            warnings.push(format!(
                "development profile CORS defaults ({}) are active on a non-loopback bind; set PHANTOM_CORS_ORIGINS (an empty value disables cross-origin access) or use --mode production",
                DEV_CORS_ORIGINS.join(", ")
            ));
        }
        if self.get_bool("PHANTOM_SIMULATION") {
            warnings.push("PHANTOM_SIMULATION=true: dashboard values it generates are SIMULATED, not measured".to_string());
        }
        if matches!(self.get("PHANTOM_INTELLIGENCE"), Some("canary") | Some("active")) {
            warnings.push(
                "PHANTOM_INTELLIGENCE canary/active are experimental; a benefit from adaptive control has not been established (docs/user/limitations.md)".to_string(),
            );
        }
        if self.get_bool("PHANTOM_ALLOW_INSECURE_HTTP") && network {
            warnings.push("PHANTOM_ALLOW_INSECURE_HTTP=true on a network bind: ingress traffic is unencrypted".to_string());
        }
        if self.get_bool("PHANTOM_FORWARD_UPSTREAM_AUTH") {
            warnings.push("PHANTOM_FORWARD_UPSTREAM_AUTH=true forwards client Authorization headers to the origin".to_string());
        }
        if self.get_bool("PHANTOM_ALLOW_LAN_ORIGIN") {
            warnings.push("PHANTOM_ALLOW_LAN_ORIGIN=true relaxes the SSRF filter for private address ranges".to_string());
        }

        // Production profile.
        if prod {
            let mut need = |ok: bool, msg: &str| {
                if !ok {
                    errors.push(ConfigError::key("PHANTOM_MODE", format!("production mode requires {}", msg)));
                }
            };
            need(require_auth, "PHANTOM_REQUIRE_AUTH=true");
            need(secret.is_some(), "PHANTOM_JWT_SECRET");
            need(!self.get_bool("PHANTOM_SIMULATION"), "PHANTOM_SIMULATION=false (simulated values must not be shown as live)");
            need(!self.get_bool("PHANTOM_ALLOW_INSECURE_HTTP"), "PHANTOM_ALLOW_INSECURE_HTTP=false");
            need(!self.get_bool("PHANTOM_ALLOW_INSECURE_UPSTREAM"), "PHANTOM_ALLOW_INSECURE_UPSTREAM=false");
            need(!self.get_bool("PHANTOM_ALLOW_INSECURE_CLUSTER"), "PHANTOM_ALLOW_INSECURE_CLUSTER=false");
            need(!self.get_bool("PHANTOM_FORWARD_UPSTREAM_AUTH"), "PHANTOM_FORWARD_UPSTREAM_AUTH=false (forwarding client credentials upstream must be a deliberate development choice)");
        }
    }

    /// Redacted effective-configuration report (also printed at startup).
    pub fn report(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "PhantomCache {} (commit {}, built {})\n",
            VERSION, BUILD_COMMIT, BUILD_DATE
        ));
        s.push_str(&format!(
            "Effective configuration (mode={}, config file: {})\n",
            self.get("PHANTOM_MODE").unwrap_or("development"),
            self.config_file.as_deref().unwrap_or("none")
        ));
        s.push_str(&format!("  {:<32} {:<28} {}\n", "KEY", "VALUE", "SOURCE"));
        for sp in KEYS {
            let e = &self.entries[sp.name];
            let shown = match (&e.value, sp.secret) {
                (None, _) => "(unset)".to_string(),
                (Some(_), true) => "<redacted>".to_string(),
                (Some(v), false) if v.is_empty() => "(empty)".to_string(),
                (Some(v), false) => v.clone(),
            };
            let mut line = format!("  {:<32} {:<28} {}", sp.name, shown, e.source.as_str());
            if let Wiring::NotWired(_) = sp.wiring {
                line.push_str(" (not honored by runtime)");
            }
            line.push('\n');
            s.push_str(&line);
        }
        let cors = self.cors_origins();
        s.push_str(&format!(
            "  effective CORS origins: {} ({}; CORS is a browser control, not authentication)\n",
            if cors.is_empty() { "none (same-origin only)".to_string() } else { cors.join(",") },
            if self.get("PHANTOM_CORS_ORIGINS").is_some() {
                "from PHANTOM_CORS_ORIGINS"
            } else if self.is_production() {
                "production default"
            } else {
                "development default"
            }
        ));
        for w in &self.warnings {
            s.push_str(&format!("  WARNING: {}\n", w));
        }
        s
    }
}

/// Filesystem preflight: every configured path must be a readable regular file.
/// Separate from `resolve` so that pure resolution stays free of I/O.
pub fn preflight(cfg: &Config) -> (Vec<ConfigError>, Vec<String>) {
    let mut errors = Vec::new();
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut warnings: Vec<String> = Vec::new();
    for s in KEYS.iter().filter(|s| matches!(s.kind, Kind::Path) && s.name != "PHANTOM_CONFIG") {
        if let Some(p) = cfg.get(s.name) {
            match std::fs::metadata(p) {
                Ok(m) if m.is_file() => {
                    if let Err(e) = std::fs::File::open(p) {
                        errors.push(ConfigError::key(s.name, format!("file '{}' is not readable: {}", p, e)));
                    }
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        if m.permissions().mode() & 0o077 != 0
                            && (s.name == "PHANTOM_TLS_KEY" || s.name == "PHANTOM_NODE_KEY")
                        {
                            warnings.push(format!(
                                "{} file '{}' is accessible by group/others; restrict to owner (chmod 600)",
                                s.name, p
                            ));
                        }
                    }
                }
                Ok(_) => errors.push(ConfigError::key(s.name, format!("'{}' is not a regular file", p))),
                Err(e) => errors.push(ConfigError::key(s.name, format!("file '{}' cannot be read: {}", p, e))),
            }
        }
    }
    (errors, warnings)
}

/// Markdown table generated from the schema (used to keep the docs in sync).
pub fn schema_markdown() -> String {
    let mut s = String::from("| Key | Type/unit | Default | Wiring | Security impact |\n|---|---|---|---|---|\n");
    for k in KEYS {
        let wiring = match k.wiring {
            Wiring::Main => "applied by main",
            Wiring::EnvExport => "env (exported)",
            Wiring::NotWired(_) => "NOT HONORED",
        };
        s.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            k.name,
            k.unit,
            k.default.map(|d| format!("`{}`", d)).unwrap_or_else(|| "unset".into()),
            wiring,
            k.security_impact
        ));
    }
    s
}

/// Debug output never contains secret values.
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut m = f.debug_map();
        for sp in KEYS {
            let e = &self.entries[sp.name];
            let shown = match (&e.value, sp.secret) {
                (Some(_), true) => Some("<redacted>".to_string()),
                (v, _) => v.clone(),
            };
            m.entry(&sp.name, &(shown, e.source.as_str()));
        }
        m.finish()
    }
}
