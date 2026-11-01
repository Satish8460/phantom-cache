//! Real gateway process management (`phantom-cache-demo`): spawn on a free loopback port, readiness,
//! metrics snapshots, kill, log tail. `kill_on_drop` guarantees no orphan if the driver unwinds.

use super::http::simple_get;
use serde_json::Value;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};

#[derive(Debug, Clone)]
pub struct CapConfig {
    pub min: usize,
    pub max: usize,
    pub p99_ceiling_ms: u64,
    pub window: usize,
}

#[derive(Debug, Clone)]
pub struct GatewaySpec {
    pub exe: PathBuf,
    pub origin: SocketAddr,
    pub max_cache_bytes: u64,
    /// `off`, `observe`, `advisory`, `canary`, `active` (PHANTOM_INTELLIGENCE).
    pub intelligence: String,
    pub cap: Option<CapConfig>,
    pub log_path: PathBuf,
}

pub struct Gateway {
    child: Child,
    pub addr: SocketAddr,
    pub pid: u32,
    pub log_path: PathBuf,
}

pub fn free_port() -> std::io::Result<u16> {
    let l = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(l.local_addr()?.port())
}

/// The environment the gateway receives (documented in the artifacts): inherited variables except any
/// `PHANTOM_*` / `ORIGIN_URL`, plus exactly the keys below.
pub fn gateway_env(spec: &GatewaySpec, port: u16) -> Vec<(String, String)> {
    let mut v = vec![
        ("PHANTOM_MODE".to_string(), "development".to_string()),
        ("PHANTOM_BIND_ADDR".to_string(), format!("127.0.0.1:{port}")),
        ("ORIGIN_URL".to_string(), format!("http://{}", spec.origin)),
        ("PHANTOM_MAX_CACHE_BYTES".to_string(), spec.max_cache_bytes.to_string()),
        ("PHANTOM_INTELLIGENCE".to_string(), spec.intelligence.clone()),
    ];
    if matches!(spec.intelligence.as_str(), "canary" | "active") {
        // The lab runs the experimental modes on purpose; the gateway rejects them unless this opt-in is set.
        v.push(("PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE".to_string(), "true".to_string()));
    }
    if let Some(c) = &spec.cap {
        v.push(("PHANTOM_ORIGIN_CAP_MAX".into(), c.max.to_string()));
        v.push(("PHANTOM_ORIGIN_CAP_MIN".into(), c.min.to_string()));
        v.push(("PHANTOM_ORIGIN_CAP_P99_CEILING_MS".into(), c.p99_ceiling_ms.to_string()));
        v.push(("PHANTOM_ORIGIN_CAP_WINDOW".into(), c.window.to_string()));
    }
    v
}

impl Gateway {
    pub async fn spawn(spec: &GatewaySpec) -> Result<Gateway, String> {
        if !spec.exe.exists() {
            return Err(format!("gateway binary not found: {}", spec.exe.display()));
        }
        let mut last_err = String::new();
        for _attempt in 0..3 {
            let port = free_port().map_err(|e| format!("free port: {e}"))?;
            let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
            let log = std::fs::File::create(&spec.log_path).map_err(|e| format!("log file: {e}"))?;
            let log2 = log.try_clone().map_err(|e| format!("log clone: {e}"))?;
            let mut cmd = Command::new(&spec.exe);
            cmd.kill_on_drop(true).stdin(std::process::Stdio::null()).stdout(log).stderr(log2);
            for (k, _) in std::env::vars() {
                if k.starts_with("PHANTOM_") || k == "ORIGIN_URL" {
                    cmd.env_remove(&k);
                }
            }
            for (k, val) in gateway_env(spec, port) {
                cmd.env(k, val);
            }
            let mut child = cmd.spawn().map_err(|e| format!("spawn {}: {e}", spec.exe.display()))?;
            let pid = child.id().unwrap_or(0);
            let t = Instant::now();
            let mut ready = false;
            while t.elapsed() < Duration::from_secs(20) {
                if let Ok(Some(st)) = child.try_wait() {
                    last_err = format!("gateway exited early ({st}); log tail:\n{}", log_tail(&spec.log_path, 30));
                    break;
                }
                if let Ok(r) = simple_get(addr, "/health", Duration::from_millis(500)).await {
                    if r.status == 200 {
                        ready = true;
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            if ready {
                return Ok(Gateway { child, addr, pid, log_path: spec.log_path.clone() });
            }
            if last_err.is_empty() {
                last_err = format!("gateway not ready within 20 s; log tail:\n{}", log_tail(&spec.log_path, 30));
            }
            let _ = child.kill().await;
        }
        Err(last_err)
    }

    pub fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Kill and reap. Returns the exit status text.
    pub async fn kill(mut self) -> String {
        let _ = self.child.kill().await;
        match self.child.wait().await {
            Ok(s) => s.to_string(),
            Err(e) => format!("wait error: {e}"),
        }
    }

    pub async fn metrics(&self) -> Result<Value, String> {
        let r = simple_get(self.addr, "/api/v1/metrics", Duration::from_secs(2))
            .await
            .map_err(|e| format!("metrics: {e}"))?;
        if r.status != 200 {
            return Err(format!("metrics status {}", r.status));
        }
        serde_json::from_slice(&r.body).map_err(|e| format!("metrics json: {e}"))
    }
}

/// Flattened gateway snapshot (every field from `/api/v1/metrics` schema 2, same payload as `/api/events`).
/// Missing values stay `null` (never zero).
pub fn flatten_snapshot(v: &Value, t_ns: u64) -> Value {
    let m = &v["metrics"];
    let g = |p: &Value| -> Value { p.get("v").cloned().unwrap_or(Value::Null) };
    let oc = &m["origin_cap"];
    serde_json::json!({
        "t_ns": t_ns,
        "schema": v["schema"],
        "origin_cap": {
            "enforcing": g(&oc["enforcing"]),
            "cap": g(&oc["cap"]),
            "in_flight": g(&oc["in_flight"]),
            "dispatched": g(&oc["dispatched"]),
            "rejections": g(&oc["origin_cap_rejections"]),
            "controller_steps": g(&oc["controller_steps"]),
        },
        "requests_total": g(&m["requests_total"]),
        "cache_hits": g(&m["cache_hits"]),
        "cache_misses": g(&m["cache_misses"]),
        "coalesced_waiters": g(&m["coalesced_waiters"]),
        "stale_served_total": g(&m["stale_served_total"]),
        "stale_cap_violations": g(&m["stale_cap_violations"]),
        "bytes_resident": g(&m["bytes_resident"]),
        "evictions_total": g(&m["evictions_total"]),
    })
}

pub fn log_tail(path: &Path, max_lines: usize) -> String {
    let s = std::fs::read(path).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
    let lines: Vec<&str> = s.lines().collect();
    let from = lines.len().saturating_sub(max_lines);
    lines[from..].join("\n")
}

/// Rewrites the log keeping only the last `max_lines` lines (call after the process has exited).
pub fn truncate_log_to_tail(path: &Path, max_lines: usize) {
    let tail = log_tail(path, max_lines);
    let _ = std::fs::write(path, tail);
}

/// True if a process with this pid is still running (best effort, Windows `tasklist` / Unix `kill -0`).
pub fn pid_alive(pid: u32) -> bool {
    if cfg!(windows) {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output();
        match out {
            Ok(o) => String::from_utf8_lossy(&o.stdout).contains(&format!("\"{pid}\"")),
            Err(_) => false,
        }
    } else {
        std::process::Command::new("kill").args(["-0", &pid.to_string()]).status().map(|s| s.success()).unwrap_or(false)
    }
}
