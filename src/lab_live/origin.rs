//! Scripted origin (raw TCP, HTTP/1.1) with a degradation schedule and per-request recording.
//!
//! Healthy -> degraded (service time x `degraded_mult` for `degrade_len_ms`, optional error rate) -> healthy,
//! all offsets relative to [`Origin::begin_schedule`]. The origin is a pure delay server (no internal
//! queue, no capacity limit): concurrency equals (arrival rate x service time). Service time is a
//! `tokio::time::sleep`; its real resolution is host dependent (recorded as `timer_overshoot` in the manifest).
//! The concurrency counter covers "request parsed" -> "just before the response is written", so the
//! number of requests being served is exact; it is decremented before the response bytes are sent so that
//! a gateway permit released on EOF can never overlap a not-yet-decremented origin slot.

use crate::lab::rng::StreamRng;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginScript {
    pub base_service_ms: u64,
    pub degraded_mult: f64,
    /// Offsets relative to `begin_schedule()`.
    pub degrade_start_ms: u64,
    pub degrade_len_ms: u64,
    /// Probability of a 503 response during the degraded interval.
    pub error_rate: f64,
    pub max_age_s: u32,
    pub stale_while_revalidate_s: Option<u32>,
    pub stale_if_error_s: Option<u32>,
    pub body_bytes: usize,
    pub seed: u64,
}

impl OriginScript {
    pub fn cache_control(&self) -> String {
        let mut s = format!("max-age={}", self.max_age_s);
        if let Some(v) = self.stale_while_revalidate_s {
            s.push_str(&format!(", stale-while-revalidate={v}"));
        }
        if let Some(v) = self.stale_if_error_s {
            s.push_str(&format!(", stale-if-error={v}"));
        }
        s
    }
}

#[derive(Debug, Clone)]
pub struct OriginRecord {
    /// ns since `Origin::start` (monotonic).
    pub arrival_ns: u64,
    /// Requests in service at this arrival, including this one.
    pub conc_at_arrival: u32,
    pub service_ns: u64,
    pub status: u16,
    pub degraded: bool,
    pub path_hash: u64,
}

struct Shared {
    script: OriginScript,
    t0: Instant,
    /// ns since t0 of the schedule origin; u64::MAX = not begun (everything healthy).
    schedule_start_ns: AtomicU64,
    conc: AtomicUsize,
    peak: AtomicUsize,
    records: Mutex<Vec<OriginRecord>>,
    rng: Mutex<StreamRng>,
    stop: AtomicBool,
}

pub struct Origin {
    pub addr: SocketAddr,
    shared: Arc<Shared>,
    accept: tokio::task::JoinHandle<()>,
}

pub struct OriginResult {
    pub records: Vec<OriginRecord>,
    /// ns since `Origin::start` at which the schedule began (None if never begun).
    pub schedule_start_ns: Option<u64>,
    pub peak_concurrency: usize,
}

pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Deterministic per-path body of exactly `n` bytes.
pub fn body_for(path: &str, n: usize) -> Vec<u8> {
    let mut rng = StreamRng::new(fnv1a(path.as_bytes()));
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let v = rng.next_u64();
        for k in 0..8 {
            if out.len() >= n {
                break;
            }
            // printable ASCII, keeps the body valid text
            out.push(b'a' + ((v >> (k * 8)) as u8 % 26));
        }
    }
    out
}

impl Origin {
    pub async fn start(script: OriginScript) -> std::io::Result<Origin> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let shared = Arc::new(Shared {
            rng: Mutex::new(StreamRng::new(script.seed ^ 0x6f72_6967_696e)),
            script,
            t0: Instant::now(),
            schedule_start_ns: AtomicU64::new(u64::MAX),
            conc: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            records: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
        });
        let sh = shared.clone();
        let accept = tokio::spawn(async move {
            loop {
                let (sock, _) = match listener.accept().await {
                    Ok(x) => x,
                    Err(_) => {
                        if sh.stop.load(Ordering::Relaxed) {
                            return;
                        }
                        tokio::time::sleep(Duration::from_millis(5)).await;
                        continue;
                    }
                };
                let sh2 = sh.clone();
                tokio::spawn(async move {
                    let _ = serve_conn(sock, sh2).await;
                });
            }
        });
        Ok(Origin { addr, shared, accept })
    }

    /// Start the degradation clock (offsets in the script are relative to this instant).
    /// Returns the schedule origin as ns since `start`.
    pub fn begin_schedule(&self) -> u64 {
        let ns = self.shared.t0.elapsed().as_nanos() as u64;
        self.shared.schedule_start_ns.store(ns, Ordering::SeqCst);
        ns
    }

    pub fn current_concurrency(&self) -> usize {
        self.shared.conc.load(Ordering::SeqCst)
    }

    pub async fn stop(self) -> OriginResult {
        self.shared.stop.store(true, Ordering::SeqCst);
        self.accept.abort();
        // Let in-flight handlers finish recording (bounded wait).
        let t = Instant::now();
        while self.shared.conc.load(Ordering::SeqCst) > 0 && t.elapsed() < Duration::from_secs(3) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let s = self.shared.schedule_start_ns.load(Ordering::SeqCst);
        let records = std::mem::take(&mut *self.shared.records.lock().unwrap());
        OriginResult {
            records,
            schedule_start_ns: if s == u64::MAX { None } else { Some(s) },
            peak_concurrency: self.shared.peak.load(Ordering::SeqCst),
        }
    }
}

async fn serve_conn(mut sock: TcpStream, sh: Arc<Shared>) -> std::io::Result<()> {
    sock.set_nodelay(true).ok();
    let mut buf = Vec::with_capacity(1024);
    let mut tmp = [0u8; 2048];
    let path = loop {
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..p]).to_string();
            let line = head.lines().next().unwrap_or("");
            let mut it = line.split_whitespace();
            let _m = it.next();
            break it.next().unwrap_or("/").to_string();
        }
        if buf.len() > 16 * 1024 {
            return Ok(());
        }
        let n = sock.read(&mut tmp).await?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
    };
    let arrival = sh.t0.elapsed();
    let conc = sh.conc.fetch_add(1, Ordering::SeqCst) + 1;
    sh.peak.fetch_max(conc, Ordering::SeqCst);
    let start_ns = sh.schedule_start_ns.load(Ordering::SeqCst);
    let degraded = if start_ns == u64::MAX {
        false
    } else {
        let off_ms = (arrival.as_nanos() as u64).saturating_sub(start_ns) / 1_000_000;
        arrival.as_nanos() as u64 >= start_ns
            && off_ms >= sh.script.degrade_start_ms
            && off_ms < sh.script.degrade_start_ms + sh.script.degrade_len_ms
    };
    let mult = if degraded { sh.script.degraded_mult } else { 1.0 };
    let service = Duration::from_micros((sh.script.base_service_ms as f64 * mult * 1000.0) as u64);
    let fail = degraded && sh.script.error_rate > 0.0 && sh.rng.lock().unwrap().next_f64() < sh.script.error_rate;
    tokio::time::sleep(service).await;
    let service_ns = sh.t0.elapsed().as_nanos() as u64 - arrival.as_nanos() as u64;
    // Release the slot BEFORE writing so a peer that observes EOF can never race a stale slot.
    sh.conc.fetch_sub(1, Ordering::SeqCst);
    let status = if fail { 503 } else { 200 };
    sh.records.lock().unwrap().push(OriginRecord {
        arrival_ns: arrival.as_nanos() as u64,
        conc_at_arrival: conc as u32,
        service_ns,
        status,
        degraded,
        path_hash: fnv1a(path.as_bytes()),
    });
    let resp = if fail {
        let body = b"origin degraded";
        let mut v = format!(
            "HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        v.extend_from_slice(body);
        v
    } else {
        let body = body_for(&path, sh.script.body_bytes);
        let mut v = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            sh.script.cache_control(),
            body.len()
        )
        .into_bytes();
        v.extend_from_slice(&body);
        v
    };
    sock.write_all(&resp).await?;
    sock.shutdown().await.ok();
    Ok(())
}
