//! Minimal HTTP/1.1 client over `tokio::net::TcpStream` (keep-alive pool, Content-Length / chunked bodies).

use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Clone, Default)]
pub struct Parsed {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body_len: usize,
    pub body: Vec<u8>,
    pub reusable: bool,
}

impl Parsed {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn eof(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, msg.to_string())
}

/// Reads one HTTP/1.1 response. `keep_body` retains the body bytes (otherwise only the length is kept).
pub async fn read_response(stream: &mut TcpStream, keep_body: bool) -> io::Result<Parsed> {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut tmp = [0u8; 4096];
    let head_end = loop {
        if let Some(p) = find(&buf, b"\r\n\r\n") {
            break p;
        }
        if buf.len() > 64 * 1024 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "response head too large"));
        }
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            return Err(eof("eof before response head"));
        }
        buf.extend_from_slice(&tmp[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or("");
    let mut parts = status_line.split_whitespace();
    let version = parts.next().unwrap_or("");
    let status: u16 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "bad status line"))?;
    let mut headers = Vec::new();
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    let mut out = Parsed { status, headers, ..Default::default() };
    let conn_close = out.header("connection").map(|v| v.eq_ignore_ascii_case("close")).unwrap_or(false);
    let mut reusable = version.eq_ignore_ascii_case("HTTP/1.1") && !conn_close;
    let mut rest: Vec<u8> = buf[head_end + 4..].to_vec();
    let chunked = out.header("transfer-encoding").map(|v| v.to_ascii_lowercase().contains("chunked")).unwrap_or(false);
    let content_length: Option<usize> = out.header("content-length").and_then(|v| v.parse().ok());
    if status == 204 || status == 304 || (100..200).contains(&status) {
        out.reusable = reusable;
        return Ok(out);
    }
    if chunked {
        let mut body = Vec::new();
        loop {
            let line_end = loop {
                if let Some(p) = find(&rest, b"\r\n") {
                    break p;
                }
                let n = stream.read(&mut tmp).await?;
                if n == 0 {
                    return Err(eof("eof in chunk size"));
                }
                rest.extend_from_slice(&tmp[..n]);
            };
            let size_str = String::from_utf8_lossy(&rest[..line_end]).to_string();
            let size = usize::from_str_radix(size_str.split(';').next().unwrap_or("").trim(), 16)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad chunk size"))?;
            rest.drain(..line_end + 2);
            while rest.len() < size + 2 {
                let n = stream.read(&mut tmp).await?;
                if n == 0 {
                    return Err(eof("eof in chunk"));
                }
                rest.extend_from_slice(&tmp[..n]);
            }
            body.extend_from_slice(&rest[..size]);
            rest.drain(..size + 2);
            if size == 0 {
                break;
            }
        }
        out.body_len = body.len();
        if keep_body {
            out.body = body;
        }
    } else if let Some(cl) = content_length {
        while rest.len() < cl {
            let n = stream.read(&mut tmp).await?;
            if n == 0 {
                return Err(eof("eof in body"));
            }
            rest.extend_from_slice(&tmp[..n]);
        }
        out.body_len = cl;
        if keep_body {
            out.body = rest[..cl].to_vec();
        }
    } else {
        // read to EOF; the connection cannot be reused
        loop {
            let n = stream.read(&mut tmp).await?;
            if n == 0 {
                break;
            }
            rest.extend_from_slice(&tmp[..n]);
        }
        reusable = false;
        out.body_len = rest.len();
        if keep_body {
            out.body = rest;
        }
    }
    out.reusable = reusable;
    Ok(out)
}

/// One-shot GET with `Connection: close` (health, metrics).
pub async fn simple_get(addr: SocketAddr, path: &str, timeout: Duration) -> io::Result<Parsed> {
    let fut = async {
        let mut s = TcpStream::connect(addr).await?;
        s.set_nodelay(true).ok();
        let req = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\nAccept: application/json\r\n\r\n");
        s.write_all(req.as_bytes()).await?;
        read_response(&mut s, true).await
    };
    tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "timeout"))?
}

/// Result of one pooled GET.
#[derive(Debug, Clone, Default)]
pub struct GetOutcome {
    /// 0 = no HTTP response (see `error`).
    pub status: u16,
    pub cache_status: String,
    pub cache_error: String,
    pub bytes: usize,
    pub error: String,
    /// ns since the caller's epoch when the request was first written (0 = never written).
    pub send_ns: u64,
}

/// Keep-alive connection pool for one target address. A request takes an idle connection or opens
/// a new one, so the number of concurrent connections follows the open-loop arrival process.
pub struct Pool {
    addr: SocketAddr,
    idle: std::sync::Mutex<Vec<TcpStream>>,
}

impl Pool {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr, idle: std::sync::Mutex::new(Vec::new()) }
    }

    fn take(&self) -> Option<TcpStream> {
        self.idle.lock().unwrap().pop()
    }

    fn give(&self, s: TcpStream) {
        let mut g = self.idle.lock().unwrap();
        if g.len() < 4096 {
            g.push(s);
        }
    }

    /// GET with a total deadline. `now_ns` returns ns since the caller's epoch (monotonic).
    pub async fn get(&self, path: &str, deadline: Duration, now_ns: &(dyn Fn() -> u64 + Sync)) -> GetOutcome {
        let mut out = GetOutcome::default();
        let fut = async {
            let req = format!(
                "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: keep-alive\r\nAccept: application/json\r\n\r\n"
            );
            for attempt in 0..2 {
                let candidate = if attempt == 0 { self.take() } else { None };
                let (mut s, reused) = match candidate {
                    Some(s) => (s, true),
                    None => {
                        let s = TcpStream::connect(self.addr).await.map_err(|e| format!("connect: {e}"))?;
                        s.set_nodelay(true).ok();
                        (s, false)
                    }
                };
                if out.send_ns == 0 {
                    out.send_ns = now_ns().max(1);
                }
                if let Err(e) = s.write_all(req.as_bytes()).await {
                    if reused {
                        continue;
                    }
                    return Err(format!("write: {e}"));
                }
                match read_response(&mut s, false).await {
                    Ok(p) => {
                        out.status = p.status;
                        out.bytes = p.body_len;
                        out.cache_status = p.header("x-cache-status").unwrap_or("").to_string();
                        out.cache_error = p.header("x-cache-error").unwrap_or("").to_string();
                        if p.reusable {
                            self.give(s);
                        }
                        return Ok(());
                    }
                    Err(e) => {
                        if reused {
                            continue; // stale idle connection: retry once on a fresh one
                        }
                        return Err(format!("read: {e}"));
                    }
                }
            }
            Err("retry exhausted".to_string())
        };
        match tokio::time::timeout(deadline, fut).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => out.error = e,
            Err(_) => out.error = "deadline".to_string(),
        }
        out
    }
}
