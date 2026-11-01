//! Phantom Lab HTTP API (G2), contract: `docs/design/lab_api_contract.md`, user doc: `docs/user/lab_api.md`.
//!
//! The API starts the SAME experiment implementations the tests and the `phantom-lab` binary use
//! (`lab::runner::execute` for the T1 simulation, `lab_live::scenario_b::run_experiment` for the T2
//! real-gateway experiment). It contains no second implementation and no canned result.
//!
//! Security properties (reviewed in `docs/user/lab_api.md`):
//! * routes exist only when a [`LabService`] was installed (config `PHANTOM_LAB_ENABLED=true`);
//! * with JWT enabled every route authenticates BEFORE any side effect or body parsing beyond the
//!   16 KiB size cap (reads: Observer, start/cancel: Operator);
//! * without JWT (development) only loopback is allowed and every request must carry an allowed
//!   `Host` (DNS-rebinding defence) and, if present, a same-origin or CORS-allowlisted `Origin`;
//! * no client-supplied path is ever joined to a filesystem path: run ids are validated against a
//!   fixed grammar, artifacts and evidence are looked up by equality against a fixed allowlist /
//!   an enumerated directory listing;
//! * the API never exposes sealed seeds: `seed_set` S/sealed and family W-SF are refused by the
//!   library validator, evidence payloads are built from whitelisted fields and scrubbed of seed
//!   value keys, and the service refuses to start if `PHANTOM_SEALED_SALT_HEX` is present in the
//!   process environment (the library validator would read it);
//! * one `RunHandle` per run owns run.json AND events.jsonl under one lock: state transitions are a
//!   monotone state machine, event ids are assigned under that lock, every line is one `write_all`.
#![allow(clippy::result_large_err)]

use super::{authenticate_request, ApiState};
use crate::lab::eval::{repo_root, sha256_file, utc_now_iso};
use crate::lab::runner::{self, RunSpec};
use crate::lab_live::scenario_b as sb;
use axum::{
    body::{to_bytes, Body, Bytes},
    extract::{Path, RawQuery, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const SCHEMA: &str = "lab-1";
pub const MAX_BODY_BYTES: usize = 16 * 1024;
pub const BODY_READ_TIMEOUT: Duration = Duration::from_secs(5);
pub const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
pub const INDEX_MAX: usize = 200;
pub const INDEX_DEFAULT_LIMIT: usize = 50;
pub const MAX_SEEDS: usize = 30;
pub const MAX_CELLS: usize = 20_000;
pub const MAX_REPETITIONS: usize = 30;
pub const MAX_SSE_STREAMS: usize = 16;
pub const HEARTBEAT_SECS: u64 = 15;
const MAX_JSON_READ: u64 = 8 * 1024 * 1024;
const MAX_LIST_ITEMS: usize = 64;
const MAX_EVENT_LINE: usize = 64 * 1024;

/// Static copy of the dated Scenario B interpretation notice (binding text, shipped with every
/// Scenario B evidence payload so a consumer cannot omit it).
pub const SCENARIO_B_NOTICE: &str = include_str!("../../docs/demo/scenario_b_interpretation_notice.md");

// ------------------------------------------------------------------------------------------
// Configuration, startup checks
// ------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LabConfig {
    /// Lab root: runs live in `<root>/runs/<run_id>/`.
    pub root: PathBuf,
    /// Directory holding retained experiments (read-only). Defaults to `root`.
    pub evidence_dir: PathBuf,
    /// Gateway binary used by `scenario_b` runs. `None` or a missing file = unavailable.
    pub gateway_exe: Option<PathBuf>,
}

impl LabConfig {
    pub fn new(root: PathBuf, gateway_exe: Option<PathBuf>) -> Self {
        Self { evidence_dir: root.clone(), root, gateway_exe }
    }
    pub fn with_evidence_dir(mut self, dir: PathBuf) -> Self {
        self.evidence_dir = dir;
        self
    }
    fn runs_dir(&self) -> PathBuf {
        self.root.join("runs")
    }
}

/// Fail-closed bind rule: the Lab API without JWT authentication may only be served on loopback.
pub fn check_lab_bind(secure: bool, addr: &SocketAddr, lab_enabled: bool) -> Result<(), String> {
    if lab_enabled && !secure && !addr.ip().is_loopback() {
        return Err("Lab API fail-closed: PHANTOM_LAB_ENABLED without JWT authentication requires a loopback bind address".to_string());
    }
    Ok(())
}

/// The library validator derives the sealed seeds from `PHANTOM_SEALED_SALT_HEX` when it is set.
/// A process that serves the Lab API must never have it in its environment.
pub fn refuse_if_sealed_salt_present(lookup: impl Fn(&str) -> bool) -> Result<(), String> {
    if lookup("PHANTOM_SEALED_SALT_HEX") {
        return Err("PHANTOM_SEALED_SALT_HEX is present in the environment: refusing to enable the Lab API in a process that could derive the sealed seeds (run the sealed evaluation in a separate process)".to_string());
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------
// Windows process helpers (hand-declared FFI, no new dependency)
// ------------------------------------------------------------------------------------------

/// Outcome of trying to reap a leftover gateway child after a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReapOutcome {
    Terminated,
    NotRunning,
    ImageMismatch,
    TimeMismatch,
    Unsupported,
    SelfOrAncestor,
    NoCreationRecord,
    AccessDenied,
}

impl ReapOutcome {
    fn as_str(&self) -> &'static str {
        match self {
            ReapOutcome::Terminated => "terminated",
            ReapOutcome::NotRunning => "not_running",
            ReapOutcome::ImageMismatch => "image_mismatch_not_touched",
            ReapOutcome::TimeMismatch => "start_time_mismatch_not_touched",
            ReapOutcome::Unsupported => "unsupported_platform_not_touched",
            ReapOutcome::SelfOrAncestor => "self_or_ancestor_not_touched",
            ReapOutcome::NoCreationRecord => "no_creation_time_recorded_not_touched",
            ReapOutcome::AccessDenied => "access_denied_not_touched",
        }
    }
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    type Handle = *mut c_void;

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct FileTime {
        lo: u32,
        hi: u32,
    }

    #[repr(C)]
    struct BasicLimit {
        per_process_user_time: i64,
        per_job_user_time: i64,
        limit_flags: u32,
        min_ws: usize,
        max_ws: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }

    #[repr(C)]
    struct ExtLimit {
        basic: BasicLimit,
        io: [u64; 6],
        process_mem: usize,
        job_mem: usize,
        peak_process: usize,
        peak_job: usize,
    }

    // JOBOBJECT_EXTENDED_LIMIT_INFORMATION is 144 bytes on x64 and 112 on x86.
    const _: () = assert!(std::mem::size_of::<ExtLimit>() == if cfg!(target_pointer_width = "64") { 144 } else { 112 });

    /// PROCESS_BASIC_INFORMATION (ntdll), used only to read the parent pid.
    #[repr(C)]
    struct ProcessBasicInformation {
        exit_status: isize,
        peb_base: usize,
        affinity_mask: usize,
        base_priority: isize,
        unique_process_id: usize,
        parent_process_id: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
        fn CloseHandle(h: Handle) -> i32;
        fn GetLastError() -> u32;
        fn QueryFullProcessImageNameW(h: Handle, flags: u32, buf: *mut u16, size: *mut u32) -> i32;
        fn GetProcessTimes(h: Handle, c: *mut FileTime, e: *mut FileTime, k: *mut FileTime, u: *mut FileTime) -> i32;
        fn TerminateProcess(h: Handle, code: u32) -> i32;
        fn CreateJobObjectW(attr: *mut c_void, name: *const u16) -> Handle;
        fn SetInformationJobObject(job: Handle, class: i32, info: *mut c_void, len: u32) -> i32;
        fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        fn GetCurrentProcess() -> Handle;
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn NtQueryInformationProcess(h: Handle, class: i32, info: *mut c_void, len: u32, ret_len: *mut u32) -> i32;
    }

    const PROCESS_TERMINATE: u32 = 0x0001;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: i32 = 9;
    const ERROR_ACCESS_DENIED: u32 = 5;

    /// Puts the current process in a job object with KILL_ON_JOB_CLOSE. Processes spawned afterwards
    /// inherit the job; when this process exits (for any reason, including a crash or kill) the
    /// last handle to the job closes and Windows terminates every process still in it.
    pub fn bind_children_to_process_lifetime() -> Result<(), String> {
        // SAFETY: plain Win32 calls with valid arguments; `info` is a correctly sized, zeroed
        // JOBOBJECT_EXTENDED_LIMIT_INFORMATION. The job handle is intentionally leaked: it must live
        // exactly as long as the process.
        unsafe {
            let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
            if job.is_null() {
                return Err("CreateJobObjectW failed".into());
            }
            let mut info: ExtLimit = std::mem::zeroed();
            info.basic.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(job, JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS, &mut info as *mut ExtLimit as *mut c_void, std::mem::size_of::<ExtLimit>() as u32) == 0 {
                CloseHandle(job);
                return Err("SetInformationJobObject failed".into());
            }
            if AssignProcessToJobObject(job, GetCurrentProcess()) == 0 {
                CloseHandle(job);
                return Err("AssignProcessToJobObject failed (the process may already be in a job that forbids nesting)".into());
            }
        }
        Ok(())
    }

    fn norm(p: &std::path::Path) -> String {
        let c = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let s = c.to_string_lossy().replace('/', "\\");
        s.strip_prefix("\\\\?\\").unwrap_or(&s).to_ascii_lowercase()
    }

    fn filetime_of(h: Handle) -> Option<u64> {
        // SAFETY: `h` is a valid process handle owned by the caller; the out-pointers are valid.
        unsafe {
            let (mut c, mut e, mut k, mut u) = (FileTime::default(), FileTime::default(), FileTime::default(), FileTime::default());
            if GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u) == 0 {
                return None;
            }
            Some(((c.hi as u64) << 32) | c.lo as u64)
        }
    }

    /// Creation time (FILETIME, 100 ns ticks since 1601) of a live process.
    pub fn process_creation_filetime(pid: u32) -> Option<u64> {
        // SAFETY: handle is checked and closed.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let r = filetime_of(h);
            CloseHandle(h);
            r
        }
    }

    fn parent_of(pid: u32) -> Option<u32> {
        // SAFETY: handle is checked and closed; `pbi` is a correctly sized out-buffer.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let mut pbi: ProcessBasicInformation = std::mem::zeroed();
            let mut ret = 0u32;
            let st = NtQueryInformationProcess(h, 0, &mut pbi as *mut _ as *mut c_void, std::mem::size_of::<ProcessBasicInformation>() as u32, &mut ret);
            CloseHandle(h);
            if st == 0 {
                Some(pbi.parent_process_id as u32)
            } else {
                None
            }
        }
    }

    /// This process and its ancestors (best effort, at most 32 levels).
    pub fn self_and_ancestors() -> Vec<u32> {
        let mut v = vec![std::process::id()];
        let mut cur = v[0];
        for _ in 0..32 {
            match parent_of(cur) {
                Some(p) if p != 0 && !v.contains(&p) => {
                    v.push(p);
                    cur = p;
                }
                _ => break,
            }
        }
        v
    }

    /// Terminates `pid` only if (1) it is neither this process nor one of its ancestors, (2) its image
    /// path equals `exe` (case, `\\?\` and 8.3 forms normalised), (3) its creation time equals the
    /// FILETIME recorded when the run started that child, and (4) that time is not before the run start.
    pub fn reap(pid: u32, exe: &std::path::Path, started_unix: i64, recorded_created_ft: Option<u64>) -> super::ReapOutcome {
        use super::ReapOutcome as R;
        if self_and_ancestors().contains(&pid) {
            return R::SelfOrAncestor;
        }
        let Some(want_ft) = recorded_created_ft else { return R::NoCreationRecord };
        // SAFETY: handle returned by OpenProcess is checked and always closed; buffers are sized.
        unsafe {
            let h = OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return if GetLastError() == ERROR_ACCESS_DENIED { R::AccessDenied } else { R::NotRunning };
            }
            let mut buf = vec![0u16; 4096];
            let mut len = buf.len() as u32;
            let image_ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len) != 0
                && norm(std::path::Path::new(&String::from_utf16_lossy(&buf[..len as usize]))) == norm(exe);
            if !image_ok {
                CloseHandle(h);
                return R::ImageMismatch;
            }
            let Some(ft) = filetime_of(h) else {
                CloseHandle(h);
                return R::TimeMismatch;
            };
            let created_unix = (ft / 10_000_000) as i64 - 11_644_473_600;
            if ft != want_ft || created_unix < started_unix - 2 {
                CloseHandle(h);
                return R::TimeMismatch;
            }
            let ok = TerminateProcess(h, 1) != 0;
            CloseHandle(h);
            if ok {
                R::Terminated
            } else {
                R::NotRunning
            }
        }
    }
}

/// Windows: ties every process this process spawns afterwards to its lifetime (job object with
/// KILL_ON_JOB_CLOSE). Other platforms: `Err` (no equivalent is implemented).
pub fn bind_children_to_process_lifetime() -> Result<(), String> {
    #[cfg(windows)]
    {
        win::bind_children_to_process_lifetime()
    }
    #[cfg(not(windows))]
    {
        Err("child-process lifetime binding is implemented for Windows only".into())
    }
}

/// See [`ReapOutcome`]. Terminates `pid` only on an exact match of the creation time recorded when the
/// run started that child (plus image path, not self/ancestor). Only Windows is implemented.
pub fn reap_leftover_gateway(pid: u32, exe: &FsPath, started_unix: i64, recorded_created_ft: Option<u64>) -> ReapOutcome {
    #[cfg(windows)]
    {
        win::reap(pid, exe, started_unix, recorded_created_ft)
    }
    #[cfg(not(windows))]
    {
        let _ = (pid, exe, started_unix, recorded_created_ft);
        ReapOutcome::Unsupported
    }
}

/// Creation time (FILETIME ticks) of a live process; `None` if unavailable (or not Windows).
pub fn process_creation_filetime(pid: u32) -> Option<u64> {
    #[cfg(windows)]
    {
        win::process_creation_filetime(pid)
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        None
    }
}

/// `YYYY-MM-DDTHH:MM:SSZ` -> unix seconds.
fn parse_iso_unix(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 {
        return None;
    }
    let n = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, m, d, hh, mm, ss) = (n(0..4)?, n(5..7)?, n(8..10)?, n(11..13)?, n(14..16)?, n(17..19)?);
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hh * 3600 + mm * 60 + ss)
}

/// Runs lock-taking work off the async threads (the run handle's locks are held during fsync/rename).
async fn blk<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.expect("blocking task panicked")
}

fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// ------------------------------------------------------------------------------------------
// Path scrubbing (server filesystem paths are not disclosed to Observers)
// ------------------------------------------------------------------------------------------

fn is_path_stop(c: char) -> bool {
    c.is_whitespace() || matches!(c, '"' | '\'' | '`' | ')' | ',' | ';' | ']' | '}' | '<' | '>')
}

/// Start of a Windows drive path (`C:\`, `C:/`) or UNC path (`\\host\share`, `\\?\C:\`) at `i`.
fn path_start(chars: &[char], i: usize) -> bool {
    let boundary = i == 0 || !chars[i - 1].is_alphanumeric();
    let drive = chars[i].is_ascii_alphabetic() && i + 2 < chars.len() && chars[i + 1] == ':' && (chars[i + 2] == '\\' || chars[i + 2] == '/');
    let unc = chars[i] == '\\' && i + 2 < chars.len() && chars[i + 1] == '\\' && chars[i + 2] != '\\' && !chars[i + 2].is_whitespace();
    boundary && (drive || unc)
}

fn replace_generic_paths(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if path_start(&chars, i) {
            let mut j = i + 3;
            while j < chars.len() && !is_path_stop(chars[j]) {
                j += 1;
            }
            out.push_str("<path>");
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// After a known prefix was replaced, the rest of that path (`<server-path>subile`) is dropped too.
fn eat_path_tails(s: &str) -> String {
    const M: &str = "<server-path>";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find(M) {
        out.push_str(&rest[..i + M.len()]);
        rest = &rest[i + M.len()..];
        if rest.chars().next().map(|c| !is_path_stop(c)).unwrap_or(false) {
            let end = rest.find(is_path_stop).unwrap_or(rest.len());
            rest = &rest[end..];
        }
    }
    out.push_str(rest);
    out
}

/// Known server prefixes (lab root, repo root, gateway exe, temp dirs; every slash form) are replaced
/// FIRST so that paths containing spaces are removed whole; then generic drive/UNC paths. A value that
/// is itself a path (not a command line) is replaced entirely, which also covers unknown paths with spaces.
pub fn scrub_path_text(s: &str, prefixes: &[String]) -> String {
    let mut out = s.to_string();
    for p in prefixes {
        out = out.replace(p.as_str(), "<server-path>");
    }
    out = eat_path_tails(&out);
    let chars: Vec<char> = out.chars().collect();
    if !chars.is_empty() && path_start(&chars, 0) && !out.contains(" --") && !out.contains('\n') && out.len() <= 520 {
        return "<path>".to_string();
    }
    replace_generic_paths(&out)
}

fn scrub_str(s: &str, prefixes: &[String]) -> String {
    scrub_path_text(s, prefixes)
}

fn scrub_value(v: &mut Value, prefixes: &[String]) {
    match v {
        Value::String(s) => *s = scrub_str(s, prefixes),
        Value::Array(a) => a.iter_mut().for_each(|x| scrub_value(x, prefixes)),
        Value::Object(m) => {
            let old = std::mem::take(m);
            for (k, mut x) in old {
                scrub_value(&mut x, prefixes);
                m.insert(scrub_str(&k, prefixes), x);
            }
        }
        _ => {}
    }
}

// ------------------------------------------------------------------------------------------
// Run handle: ONE lock for run.json + events.jsonl
// ------------------------------------------------------------------------------------------

fn is_terminal(state: &str) -> bool {
    matches!(state, "completed" | "failed" | "cancelled" | "interrupted")
}

/// `YYYYMMDDTHHMMSSZ-xxxxxxxx` (UTC timestamp + 8 random hex digits; never contains a seed).
fn valid_run_id(id: &str) -> bool {
    let b = id.as_bytes();
    if b.len() != 25 || b[8] != b'T' || b[15] != b'Z' || b[16] != b'-' {
        return false;
    }
    b[..8].iter().all(|c| c.is_ascii_digit())
        && b[9..15].iter().all(|c| c.is_ascii_digit())
        && b[17..].iter().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
}

fn new_run_id() -> String {
    let ts: String = utc_now_iso().chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    format!("{}-{:08x}", ts, rand::random::<u32>())
}

/// Atomic JSON write: unique temp name created with `create_new` (never follows a pre-planted
/// file or symlink), flushed, renamed over `path`. A failure is returned, never swallowed.
fn write_json_atomic(path: &FsPath, v: &Value) -> std::io::Result<()> {
    let tmp = path.with_file_name(format!(
        "{}.{:08x}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("x"),
        rand::random::<u32>()
    ));
    {
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        f.write_all(serde_json::to_string_pretty(v).unwrap_or_default().as_bytes())?;
        f.sync_all()?;
    }
    let mut last = Ok(());
    for _ in 0..5 {
        match std::fs::rename(&tmp, path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last = Err(e);
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    let _ = std::fs::remove_file(&tmp);
    last
}

fn is_reparse(md: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        md.file_type().is_symlink() || md.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    {
        md.file_type().is_symlink()
    }
}

/// A real directory: not a symlink, junction or other reparse point.
fn plain_dir(p: &FsPath) -> bool {
    std::fs::symlink_metadata(p).map(|m| m.is_dir() && !is_reparse(&m)).unwrap_or(false)
}

/// Opens a regular file (no symlink/reparse point) and takes type and size from the OPEN HANDLE,
/// so the checked object is the one that is read.
fn open_regular(p: &FsPath) -> Option<(std::fs::File, u64)> {
    let md = std::fs::symlink_metadata(p).ok()?;
    if !md.is_file() || is_reparse(&md) {
        return None;
    }
    let f = std::fs::File::open(p).ok()?;
    let fm = f.metadata().ok()?;
    if !fm.is_file() || is_reparse(&fm) {
        return None;
    }
    Some((f, fm.len()))
}

fn read_json_capped(path: &FsPath) -> Option<Value> {
    let (f, len) = open_regular(path)?;
    if len > MAX_JSON_READ {
        return None;
    }
    let mut buf = Vec::new();
    f.take(MAX_JSON_READ + 1).read_to_end(&mut buf).ok()?;
    if buf.len() as u64 > MAX_JSON_READ {
        return None;
    }
    serde_json::from_slice(&buf).ok()
}

fn state_rank(s: &str) -> u8 {
    match s {
        "queued" => 0,
        "running" => 1,
        "cancelling" => 2,
        _ => 3,
    }
}

struct RunInner {
    v: Value,
    events: std::fs::File,
    seq: u64,
    persist_errors: u64,
}

pub struct RunHandle {
    id: String,
    dir: PathBuf,
    cancel: Arc<AtomicBool>,
    inner: Mutex<RunInner>,
    tx: tokio::sync::watch::Sender<u64>,
    prefixes: Vec<String>,
    terminal: AtomicBool,
}

impl RunHandle {
    fn create(dir: PathBuf, id: String, v: Value, cancel: Arc<AtomicBool>, prefixes: Vec<String>) -> std::io::Result<Arc<Self>> {
        std::fs::create_dir_all(&dir)?;
        let events = std::fs::OpenOptions::new().create_new(true).append(true).open(dir.join("events.jsonl"))?;
        let (tx, _rx) = tokio::sync::watch::channel(0u64);
        let h = Arc::new(Self {
            id,
            dir,
            cancel,
            inner: Mutex::new(RunInner { v, events, seq: 0, persist_errors: 0 }),
            tx,
            prefixes,
            terminal: AtomicBool::new(false),
        });
        {
            let mut g = h.inner.lock().unwrap_or_else(|e| e.into_inner());
            let tier = g.v["tier"].clone();
            h.emit_locked(&mut g, "state", json!({"state": "queued", "tier": tier}));
            h.persist_locked(&mut g)?;
        }
        Ok(h)
    }

    /// Id assigned under the lock; the whole line is ONE `write_all` on an append-mode file.
    fn emit_locked(&self, g: &mut RunInner, event: &str, mut data: Value) {
        scrub_value(&mut data, &self.prefixes); // no event can carry a server path
        g.seq += 1;
        let mut line = json!({"id": g.seq, "event": event, "ts": utc_now_iso(), "data": data}).to_string();
        line.push('\n');
        if g.events.write_all(line.as_bytes()).and_then(|_| g.events.flush()).is_err() {
            g.persist_errors += 1;
            eprintln!("[lab] could not append event {} of run {}", g.seq, self.id);
        }
        self.tx.send_modify(|x| *x += 1);
    }


    fn persist_locked(&self, g: &mut RunInner) -> std::io::Result<()> {
        g.v["persistence_errors"] = json!(g.persist_errors);
        let r = write_json_atomic(&self.dir.join("run.json"), &g.v);
        if let Err(ref e) = r {
            g.persist_errors += 1;
            eprintln!("[lab] could not persist run.json of run {}: {}", self.id, e.kind());
        }
        r
    }

    /// queued -> running. No effect in any other state.
    fn begin_running(&self) {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if g.v["state"] == "queued" {
            g.v["state"] = json!("running");
            g.v["started_utc"] = json!(utc_now_iso());
            self.emit_locked(&mut g, "state", json!({"state": "running"}));
            let _ = self.persist_locked(&mut g);
        }
    }

    /// Progress never touches the state field.
    fn progress(&self, data: Value, gateway_pid: Option<u32>) {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if is_terminal(g.v["state"].as_str().unwrap_or("")) {
            return;
        }
        g.v["progress"] = json!({"done": data["done"], "total": data["total"]});
        if let Some(p) = gateway_pid.filter(|p| *p != 0) {
            if g.v["gateway_pid"].as_u64() != Some(p as u64) {
                g.v["gateway_pid"] = json!(p);
                // creation time of the child, recorded now: startup reaping requires an exact match
                g.v["gateway_created_ft"] = match process_creation_filetime(p) {
                    Some(ft) => json!(ft.to_string()),
                    None => Value::Null,
                };
            }
        }
        self.emit_locked(&mut g, "progress", data);
        let _ = self.persist_locked(&mut g);
    }

    /// queued|running -> cancelling (once). Returns whether this call changed the state.
    fn request_cancel(&self) -> bool {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let st = g.v["state"].as_str().unwrap_or("").to_string();
        if state_rank(&st) < 2 {
            g.v["state"] = json!("cancelling");
            self.emit_locked(&mut g, "state", json!({"state": "cancelling"}));
            let _ = self.persist_locked(&mut g);
            true
        } else {
            false
        }
    }

    fn state(&self) -> String {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).v["state"].as_str().unwrap_or("").to_string()
    }

    /// Terminal transition, idempotent (a terminal run is never overwritten). The terminal
    /// `run.json` is written BEFORE the single run slot is released, and both happen under the
    /// service's active lock, so a client that sees the terminal state and starts the next run
    /// waits for the slot instead of getting a spurious 409.
    fn finish(
        &self,
        svc: &LabService,
        state: &str,
        exit_code: Option<i32>,
        result: Option<Value>,
        error: Option<(&str, &str)>,
        artifact_index: Option<Value>,
    ) {
        let mut active = svc.active.lock().unwrap_or_else(|e| e.into_inner());
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if is_terminal(g.v["state"].as_str().unwrap_or("")) {
            return;
        }
        if let Some(r) = result {
            self.emit_locked(&mut g, "result", r);
        }
        if let Some((code, msg)) = error {
            self.emit_locked(&mut g, "error", json!({"code": code, "message": msg}));
        }
        self.emit_locked(&mut g, "state", json!({"state": state}));
        self.emit_locked(&mut g, "done", json!({"state": state, "exit_code": exit_code}));
        g.v["state"] = json!(state);
        g.v["ended_utc"] = json!(utc_now_iso());
        g.v["exit_code"] = json!(exit_code);
        if let Some((code, msg)) = error {
            g.v["error"] = json!({"code": code, "message": msg});
        }
        if let Some(ix) = artifact_index {
            g.v["artifact_index"] = ix;
        }
        if self.persist_locked(&mut g).is_err() {
            // one retry; the failure is counted in `persistence_errors` and logged either way
            let _ = self.persist_locked(&mut g);
        }
        if active.as_ref().map(|a| a.id == self.id).unwrap_or(false) {
            *active = None;
        }
        self.terminal.store(true, Ordering::Release);
    }
}

// ------------------------------------------------------------------------------------------
// Service
// ------------------------------------------------------------------------------------------

pub struct LabService {
    cfg: LabConfig,
    active: Mutex<Option<Arc<RunHandle>>>,
    sse_open: AtomicUsize,
    bind: OnceLock<SocketAddr>,
    tls: AtomicBool,
}

struct SsePermit(Arc<LabService>);
impl Drop for SsePermit {
    fn drop(&mut self) {
        self.0.sse_open.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Backstop: clears the slot if a worker ends without a terminal transition.
struct ActiveGuard<'a>(&'a LabService, String);
impl Drop for ActiveGuard<'_> {
    fn drop(&mut self) {
        let mut a = self.0.active.lock().unwrap_or_else(|e| e.into_inner());
        if a.as_ref().map(|x| x.id == self.1).unwrap_or(false) {
            *a = None;
        }
    }
}

struct TailInfo {
    last_id: u64,
    done: bool,
    ends_with_newline: bool,
}

/// Bounded read of the end of events.jsonl (last 64 KiB): last valid event id, whether a `done` event
/// is present, and whether the file ends with a newline (a crash can leave a torn last line).
fn tail_info(path: &FsPath) -> TailInfo {
    let mut info = TailInfo { last_id: 0, done: false, ends_with_newline: true };
    let Ok(mut f) = std::fs::File::open(path) else { return info };
    let Ok(len) = f.metadata().map(|m| m.len()) else { return info };
    let start = len.saturating_sub(64 * 1024);
    if f.seek(SeekFrom::Start(start)).is_err() {
        return info;
    }
    let mut buf = Vec::new();
    if f.take(64 * 1024).read_to_end(&mut buf).is_err() {
        return info;
    }
    info.ends_with_newline = buf.last().map(|b| *b == b'\n').unwrap_or(true);
    for (i, piece) in buf.split(|b| *b == b'\n').enumerate() {
        if i == 0 && start > 0 {
            continue; // possibly a partial line
        }
        if let Ok(v) = serde_json::from_slice::<Value>(piece) {
            info.last_id = info.last_id.max(v["id"].as_u64().unwrap_or(0));
            if v["event"] == "done" {
                info.done = true;
            }
        }
    }
    info
}

impl LabService {
    /// Creates the run directory, refuses to run with the sealed salt in the environment, reaps
    /// leftover gateway children and rewrites every non-terminal run (among the newest `INDEX_MAX`)
    /// to `interrupted`. Nothing is resumed; nothing is reported completed.
    pub fn new(cfg: LabConfig) -> std::io::Result<Arc<Self>> {
        refuse_if_sealed_salt_present(|k| std::env::var_os(k).is_some())
            .map_err(|m| std::io::Error::new(std::io::ErrorKind::InvalidInput, m))?;
        std::fs::create_dir_all(cfg.runs_dir())?;
        let svc = Arc::new(Self { cfg, active: Mutex::new(None), sse_open: AtomicUsize::new(0), bind: OnceLock::new(), tls: AtomicBool::new(false) });
        svc.recover_interrupted();
        Ok(svc)
    }

    /// Records the address the server listens on (used by the Host check).
    pub fn set_bind_addr(&self, addr: SocketAddr) {
        let _ = self.bind.set(addr);
    }

    /// Records whether ingress is TLS (the only scheme an acceptable `Origin` may carry).
    pub fn set_tls(&self, tls: bool) {
        self.tls.store(tls, Ordering::SeqCst);
    }

    fn list_run_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = match std::fs::read_dir(self.cfg.runs_dir()) {
            Ok(rd) => rd.flatten().filter_map(|e| e.file_name().into_string().ok()).filter(|n| valid_run_id(n)).collect(),
            Err(_) => Vec::new(),
        };
        ids.sort_by(|a, b| b.cmp(a));
        ids.truncate(INDEX_MAX);
        ids
    }

    fn run_dir(&self, id: &str) -> Option<PathBuf> {
        if !valid_run_id(id) {
            return None;
        }
        let p = self.cfg.runs_dir().join(id);
        if plain_dir(&p) {
            Some(p)
        } else {
            None
        }
    }

    fn scrub_prefixes(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        let mut add = |p: &FsPath| {
            let s = p.to_string_lossy().to_string();
            if s.len() > 3 {
                v.push(format!("\\\\?\\{}", s.replace('/', "\\")));
                v.push(s.replace('/', "\\"));
                v.push(s.replace('\\', "/"));
                v.push(s);
            }
        };
        add(&self.cfg.root);
        add(&self.cfg.evidence_dir);
        add(&repo_root());
        if let Some(p) = &self.cfg.gateway_exe {
            add(p);
        }
        add(&std::env::temp_dir());
        v.sort_by_key(|s| std::cmp::Reverse(s.len()));
        v.dedup();
        v
    }

    fn recover_interrupted(&self) {
        for id in self.list_run_ids() {
            let Some(dir) = self.run_dir(&id) else { continue };
            let path = dir.join("run.json");
            let Some(mut v) = read_json_capped(&path) else { continue };
            let st = v["state"].as_str().unwrap_or("").to_string();
            if !matches!(st.as_str(), "running" | "queued" | "cancelling") {
                continue;
            }
            let mut recovery = serde_json::Map::new();
            if let (Some(pid), Some(exe)) = (v["gateway_pid"].as_u64(), self.cfg.gateway_exe.as_ref()) {
                let started = v["started_utc"].as_str().or(v["created_utc"].as_str()).and_then(parse_iso_unix).unwrap_or(0);
                let ft = v["gateway_created_ft"].as_str().and_then(|x| x.parse::<u64>().ok());
                let out = reap_leftover_gateway(pid as u32, exe, started, ft);
                recovery.insert("gateway_pid".into(), json!(pid));
                recovery.insert("gateway_reap".into(), json!(out.as_str()));
            }
            v["state"] = json!("interrupted");
            v["ended_utc"] = json!(utc_now_iso());
            v["exit_code"] = Value::Null;
            v["error"] = json!({"code": "interrupted", "message": "the gateway process stopped or restarted while this run was active; the run was not resumed and has no result"});
            v["recovery"] = Value::Object(recovery);
            let ti = tail_info(&dir.join("events.jsonl"));
            let (last, has_done) = (ti.last_id, ti.done);
            if has_done {
                v["recovery"]["note"] = json!("events.jsonl already contained a done event; no further event was appended");
            }
            if write_json_atomic(&path, &v).is_err() {
                eprintln!("[lab] could not rewrite run {id} to interrupted");
                continue;
            }
            if !has_done {
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("events.jsonl")) {
                    let mut text = String::new();
                    if !ti.ends_with_newline {
                        text.push('\n'); // torn tail: never glue a new event onto a partial line
                    }
                    for (i, (ev, d)) in [
                        ("state", json!({"state": "interrupted", "source": "server_recovery_on_startup"})),
                        ("done", json!({"state": "interrupted", "source": "server_recovery_on_startup"})),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        text.push_str(&json!({"id": last + 1 + i as u64, "event": ev, "ts": utc_now_iso(), "data": d}).to_string());
                        text.push('\n');
                    }
                    let _ = f.write_all(text.as_bytes());
                }
            }
        }
    }

    fn handle_for(&self, id: &str) -> Option<Arc<RunHandle>> {
        self.active.lock().unwrap_or_else(|e| e.into_inner()).as_ref().filter(|a| a.id == id).cloned()
    }

    fn active_id(&self) -> Option<String> {
        self.active.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|a| a.id.clone())
    }

    fn gateway_status(&self) -> Result<PathBuf, String> {
        match &self.cfg.gateway_exe {
            None => Err("no gateway binary configured (PHANTOM_LAB_GATEWAY_EXE)".to_string()),
            Some(p) => match std::fs::metadata(p) {
                Ok(m) if m.is_file() => Ok(p.clone()),
                _ => Err("the configured gateway binary does not exist or is not a file".to_string()),
            },
        }
    }

    /// Computed once, at run end, off the request path. JSON artifacts are hashed as SERVED
    /// (server paths scrubbed).
    fn build_artifact_index(&self, dir: &FsPath, kind: &str) -> Value {
        let prefixes = self.scrub_prefixes();
        let mut ix = serde_json::Map::new();
        for (sub, name, _) in artifact_allowlist(kind) {
            if name == "run.json" || name == "events.jsonl" {
                continue;
            }
            let Some(p) = artifact_path(dir, sub, name) else { continue };
            let Some(len) = regular_file(&p) else { continue };
            if len > MAX_ARTIFACT_BYTES {
                continue;
            }
            let sha = if name.ends_with(".json") {
                served_json_bytes(&p, &prefixes).map(|b| crate::lab::rng::sha256_hex(&b))
            } else {
                sha256_file(&p).ok() // regular file verified just above
            };
            if let Some(s) = sha {
                ix.insert(name.to_string(), json!({"bytes": len, "sha256": s}));
            }
        }
        Value::Object(ix)
    }
}

/// Reads a JSON artifact and returns the bytes that are served: server paths scrubbed.
fn served_json_bytes(p: &FsPath, prefixes: &[String]) -> Option<Vec<u8>> {
    let mut v = read_json_capped(p)?;
    scrub_value(&mut v, prefixes);
    serde_json::to_vec_pretty(&v).ok()
}

// ------------------------------------------------------------------------------------------
// Router and the anonymous-mode Host/Origin guard
// ------------------------------------------------------------------------------------------

#[derive(Clone)]
struct LabCtx {
    api: Arc<ApiState>,
    lab: Arc<LabService>,
    cors_origins: Arc<Vec<String>>,
}

pub fn router(api: Arc<ApiState>, lab: Arc<LabService>, cors_origins: Vec<String>) -> Router {
    let ctx = LabCtx { api, lab, cors_origins: Arc::new(cors_origins) };
    Router::new()
        .route("/api/v1/lab/capabilities", get(capabilities))
        .route("/api/v1/lab/evidence", get(evidence_index))
        .route("/api/v1/lab/evidence/:id", get(evidence_detail))
        .route("/api/v1/lab/runs", post(runs_start).get(runs_list))
        .route("/api/v1/lab/runs/:id", get(run_get))
        .route("/api/v1/lab/runs/:id/cancel", post(run_cancel))
        .route("/api/v1/lab/runs/:id/events", get(run_events))
        .route("/api/v1/lab/runs/:id/artifacts", get(artifacts_list))
        .route("/api/v1/lab/runs/:id/artifacts/:name", get(artifact_get))
        .layer(from_fn_with_state(ctx.clone(), host_origin_guard))
        .with_state(ctx)
}

/// Splits a Host header value into (lowercase name incl. brackets for IPv6, optional port).
fn parse_host(h: &str) -> Option<(String, Option<u16>)> {
    if h.is_empty() || h.len() > 255 || h.bytes().any(|b| b.is_ascii_whitespace() || matches!(b, b'@' | b'/' | b'\\' | b'?' | b'#')) {
        return None;
    }
    let (name, rest) = if h.starts_with('[') {
        let end = h.find(']')?;
        (&h[..=end], &h[end + 1..])
    } else {
        match h.find(':') {
            Some(i) => (&h[..i], &h[i..]),
            None => (h, ""),
        }
    };
    let port = match rest {
        "" => None,
        r => Some(r.strip_prefix(':')?.parse::<u16>().ok()?),
    };
    Some((name.to_ascii_lowercase(), port))
}

/// Anonymous (no JWT) mode only: DNS-rebinding and cross-site defence.
async fn host_origin_guard(State(ctx): State<LabCtx>, req: Request, next: Next) -> Response {
    if secure_mode(&ctx) {
        return next.run(req).await;
    }
    let single_host = req.headers().get_all(header::HOST).iter().count() == 1;
    let host_hdr = req.headers().get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let bind = ctx.lab.bind.get().copied();
    let host_ok = match parse_host(&host_hdr).filter(|_| single_host) {
        None => false,
        Some((name, port)) => {
            let mut allowed = vec!["127.0.0.1".to_string(), "localhost".to_string(), "[::1]".to_string()];
            if let Some(b) = bind {
                if !b.ip().is_unspecified() {
                    allowed.push(match b.ip() {
                        std::net::IpAddr::V4(a) => a.to_string(),
                        std::net::IpAddr::V6(a) => format!("[{a}]"),
                    });
                }
            }
            allowed.contains(&name) && bind.map(|b| port == Some(b.port())).unwrap_or(true)
        }
    };
    if !host_ok {
        return err(StatusCode::MISDIRECTED_REQUEST, "bad_host", "the Host header is not allowed in anonymous mode");
    }
    if let Some(o) = req.headers().get(header::ORIGIN) {
        let o = o.to_str().unwrap_or("").to_ascii_lowercase();
        let scheme = if ctx.lab.tls.load(Ordering::SeqCst) { "https" } else { "http" };
        let same = o == format!("{scheme}://{}", host_hdr.to_ascii_lowercase());
        if !same && !ctx.cors_origins.iter().any(|c| c.to_ascii_lowercase() == o) {
            return err(StatusCode::FORBIDDEN, "bad_origin", "the Origin header is not allowed in anonymous mode");
        }
    }
    next.run(req).await
}

// ------------------------------------------------------------------------------------------
// Response helpers
// ------------------------------------------------------------------------------------------

fn with_schema(mut v: Value) -> Value {
    if let Some(o) = v.as_object_mut() {
        o.insert("schema".into(), json!(SCHEMA));
    }
    v
}

fn ok_json(status: StatusCode, v: Value) -> Response {
    (status, [(header::CACHE_CONTROL, "no-store")], Json(with_schema(v))).into_response()
}

fn err(status: StatusCode, code: &str, msg: &str) -> Response {
    ok_json(status, json!({"error": {"code": code, "message": msg}}))
}

fn not_found() -> Response {
    err(StatusCode::NOT_FOUND, "not_found", "not found")
}

fn unavailable_metric(reason: &str) -> Value {
    json!({"unavailable": true, "reason": reason})
}

#[allow(clippy::result_large_err)]
fn authorize(ctx: &LabCtx, headers: &HeaderMap, role: crate::security::Role) -> Result<Option<u64>, Response> {
    if let Some(ref sec) = ctx.api.security {
        if sec.require_auth_for_control {
            let claims = authenticate_request(headers, sec, role)?;
            return Ok(Some(claims.exp));
        }
    }
    Ok(None)
}

fn secure_mode(ctx: &LabCtx) -> bool {
    ctx.api.security.as_ref().map(|s| s.require_auth_for_control).unwrap_or(false)
}

/// Strict `limit`/`offset` parser (same rules as the audit endpoint, max 200).
fn parse_page(raw: Option<&str>) -> Result<(usize, usize), &'static str> {
    let (mut limit, mut offset): (Option<&str>, Option<&str>) = (None, None);
    for pair in raw.unwrap_or("").split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k {
                "limit" if limit.is_none() => limit = Some(v),
                "offset" if offset.is_none() => offset = Some(v),
                _ => {}
            }
        } else if pair == "limit" || pair == "offset" {
            return Err("limit/offset need a value");
        }
    }
    let digits = |s: &str| !s.is_empty() && s.len() <= 18 && s.bytes().all(|b| b.is_ascii_digit());
    let limit = match limit {
        None => INDEX_DEFAULT_LIMIT,
        Some(v) if digits(v) => v.parse::<usize>().map_err(|_| "limit must be an integer")?,
        Some(_) => return Err("limit must be a non-negative decimal integer"),
    };
    if limit == 0 || limit > INDEX_MAX {
        return Err("limit must be between 1 and 200");
    }
    let offset = match offset {
        None => 0,
        Some(v) if digits(v) => v.parse::<usize>().map_err(|_| "offset must be an integer")?,
        Some(_) => return Err("offset must be a non-negative decimal integer"),
    };
    Ok((limit, offset))
}

fn bad_page(msg: &str) -> Response {
    err(StatusCode::BAD_REQUEST, "bad_request", msg)
}

// ------------------------------------------------------------------------------------------
// GET /capabilities
// ------------------------------------------------------------------------------------------

async fn capabilities(State(ctx): State<LabCtx>, headers: HeaderMap) -> Response {
    if let Err(r) = authorize(&ctx, &headers, crate::security::Role::Observer) {
        return r;
    }
    let svc0 = ctx.lab.clone();
    let active_run = blk(move || svc0.active_id()).await;
    let build_profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let seed_sets: Vec<Value> = runner::list_seed_sets().into_iter().map(|(n, c)| json!({"name": n, "size": c})).collect();
    let frozen_ok = repo_root().join("lab-results/tuning_v2_2c/frozen_arm_params.json").is_file();
    let sb_kind = match ctx.lab.gateway_status() {
        Ok(exe) => {
            let (label, reasons) = sb::label_for(&sb::ScenarioBConfig::default_experiment(exe.clone(), PathBuf::new()));
            json!({
                "available": true,
                "tier": sb::TIER,
                "driver_build_profile": build_profile,
                "gateway_profile_inferred_from_path": sb::gateway_profile(&exe),
                "label_for_default_experiment": label,
                "label_reasons": reasons,
                "label_meaning": sb::LABEL_VALID_MEANING,
                "arms": ["BASE", "STATIC-CAP", "AIMD-CAP"],
                "limits": {
                    "repetitions": {"min": 1, "max": MAX_REPETITIONS, "default": 10, "valid_label_needs_at_least": 10},
                    "phase_seconds": {"min": 1, "max": 60},
                    "rate_rps": {"min": 1, "max": 2000, "default": 300},
                    "n_keys": {"min": 10, "max": 100000, "default": 5000},
                    "caps": {"min": 1, "max": 1000}
                }
            })
        }
        Err(reason) => json!({
            "available": false,
            "reason": reason,
            "tier": sb::TIER,
            "driver_build_profile": build_profile,
        }),
    };
    ok_json(
        StatusCode::OK,
        json!({
            "security_mode": if secure_mode(&ctx) { "jwt" } else { "anonymous_dev_loopback_only" },
            "auth": {
                "reads": "Observer (telemetry:read)",
                "start_and_cancel": "Operator (scenario:operate)",
                "sse": "consume with fetch streaming and an Authorization header; EventSource URL form is not supported (no credentials in URLs)"
            },
            "max_concurrent_runs": 1,
            "active_run": active_run,
            "limits": {
                "body_bytes": MAX_BODY_BYTES, "artifact_bytes": MAX_ARTIFACT_BYTES, "index_max": INDEX_MAX,
                "heartbeat_seconds": HEARTBEAT_SECS
            },
            "kinds": {
                "t1_matrix": {
                    "available": true,
                    "tier": "SIMULATED",
                    "driver_build_profile": build_profile,
                    "seed_sets": seed_sets,
                    "seed_sets_note": "only T (tuning) and D (development); the sealed set is refused and its values are never exposed",
                    "scenarios": runner::list_scenarios(),
                    "families": runner::list_families(),
                    "arms": runner::list_arms(),
                    "profiles": ["smoke", "standard"],
                    "arm_params": {"untuned": true, "frozen": frozen_ok},
                    "limits": {"max_seeds": MAX_SEEDS, "max_cells": MAX_CELLS}
                },
                "scenario_b": sb_kind
            }
        }),
    )
}

// ------------------------------------------------------------------------------------------
// Evidence (read-only view of retained experiments)
// ------------------------------------------------------------------------------------------

struct EvidenceEntry {
    id: String,
    dir: PathBuf,
    kind: &'static str,
}

fn is_plain_dir(p: &FsPath) -> bool {
    std::fs::symlink_metadata(p).map(|m| m.is_dir() && !m.file_type().is_symlink()).unwrap_or(false)
}

fn classify_top(name: &str) -> Option<&'static str> {
    if name.starts_with("m6_sealed_") {
        Some("m6_sealed_evaluation")
    } else if name.starts_with("m6_dryrun_") {
        Some("m6_dry_run")
    } else if name.starts_with("tuning") {
        Some("tuning")
    } else if name.starts_with("smoke_") {
        Some("smoke")
    } else {
        None
    }
}

fn scan_evidence(cfg: &LabConfig) -> Vec<EvidenceEntry> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(&cfg.evidence_dir) else { return out };
    for e in rd.flatten().take(2000) {
        let Ok(name) = e.file_name().into_string() else { continue };
        let p = e.path();
        if !is_plain_dir(&p) || name == "runs" {
            continue;
        }
        if name == "scenario_b" {
            if let Ok(rd2) = std::fs::read_dir(&p) {
                for e2 in rd2.flatten().take(2000) {
                    let Ok(n2) = e2.file_name().into_string() else { continue };
                    let p2 = e2.path();
                    if is_plain_dir(&p2) && n2.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-') {
                        out.push(EvidenceEntry { id: format!("scenario_b.{n2}"), dir: p2, kind: "scenario_b" });
                    }
                }
            }
        } else if let Some(kind) = classify_top(&name) {
            if name.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.')) {
                out.push(EvidenceEntry { id: name, dir: p, kind });
            }
        }
    }
    out.sort_by(|a, b| b.id.cmp(&a.id));
    out.truncate(INDEX_MAX);
    out
}

const SEED_KEYS: [&str; 5] = ["seed_values", "seeds", "seed_list", "numeric_seeds", "seed_labels"];

/// Defence in depth: remove any key that could carry seed values from a retained document.
fn scrub_seeds(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.retain(|k, _| !SEED_KEYS.contains(&k.as_str()));
            for x in m.values_mut() {
                scrub_seeds(x);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(scrub_seeds),
        _ => {}
    }
}

fn artifact_names(dir: &FsPath) -> Vec<Value> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten().take(500) {
            if let (Ok(md), Ok(n)) = (std::fs::symlink_metadata(e.path()), e.file_name().into_string()) {
                if md.is_file() {
                    v.push(json!({"name": n, "bytes": md.len()}));
                }
            }
        }
    }
    v.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    v
}

fn evidence_summary(en: &EvidenceEntry, detail: bool) -> Value {
    let manifest = read_json_capped(&en.dir.join("manifest.json"));
    let mut o = serde_json::Map::new();
    o.insert("id".into(), json!(en.id));
    o.insert("kind".into(), json!(en.kind));
    let m_str = |k: &str| manifest.as_ref().and_then(|m| m[k].as_str()).map(|s| s.to_string());
    let started = m_str("started_utc");
    let ended = m_str("ended_utc");
    o.insert("started_utc".into(), started.map(Value::from).unwrap_or_else(|| unavailable_metric("not recorded in the manifest")));
    o.insert("ended_utc".into(), ended.map(Value::from).unwrap_or_else(|| unavailable_metric("not recorded in the manifest")));
    o.insert("read_only".into(), json!(true));
    let (tier, original_tier) = match en.kind {
        "scenario_b" => ("RETAINED_MEASURED_LOOPBACK", Some(sb::TIER.to_string())),
        "tuning" => ("RETAINED_SIMULATED", None),
        _ => ("RETAINED_SIMULATED", m_str("tier").or_else(|| Some("SIMULATED".into()))),
    };
    o.insert("tier".into(), json!(tier));
    o.insert("original_tier".into(), original_tier.map(Value::from).unwrap_or_else(|| unavailable_metric("no manifest tier recorded")));

    let mut verdict_snapshot = unavailable_metric("this retained artifact set records no verdict");
    let mut verdict_full: Option<Value> = None;
    let mut extra = serde_json::Map::new();
    match en.kind {
        "m6_sealed_evaluation" | "m6_dry_run" => {
            if let Some(mut vj) = read_json_capped(&en.dir.join("verdict.json")) {
                scrub_seeds(&mut vj);
                verdict_snapshot = json!({
                    "H1": vj["H1"]["verdict"].clone(),
                    "H2": vj["H2"]["status"].clone(),
                    "H3": vj["H3"]["status"].clone(),
                    "H4": vj["H4"]["status"].clone(),
                    "H5_H6": vj["H5_H6"]["status"].clone(),
                    "banner": vj["banner"].clone(),
                    "protocol_version": vj["protocol_version"].clone(),
                    "cells": vj["cells"].clone(),
                    "expected_cells": vj["expected_cells"].clone(),
                    "recorded_exactly": "values are copied from verdict.json without interpretation",
                });
                verdict_full = Some(vj);
            }
            if en.kind == "m6_sealed_evaluation" {
                extra.insert("sealed".into(), json!({"seed_set": "S", "seed_values": null, "note": "seed values of the sealed set are never read, derived or exposed by this API"}));
            }
        }
        "scenario_b" => {
            if let Some(sj) = read_json_capped(&en.dir.join("summary.json")) {
                // U-01: a null/absent `verdicts` (incomplete run) must stay null, never become an empty object.
                let verdicts: Value = match sj["verdicts"].as_object() {
                    Some(m) => Value::Object(m.iter().map(|(k, v)| (k.clone(), json!({"outcome": v["outcome"], "reasons": v["reasons"]}))).collect()),
                    None => Value::Null,
                };
                verdict_snapshot = json!({
                    "label": sj["label"].clone(),
                    "label_planned": sj["label_planned"].clone(),
                    "label_reasons": sj["label_reasons"].clone(),
                    "status": sj["status"].clone(),
                    "run_completeness": sj["run_completeness"].clone(),
                    "verdicts_status": sj["verdicts_status"].clone(),
                    "verdicts_reason": sj["verdicts_reason"].clone(),
                    "mechanism_h_b1_all_pass": sj["mechanism_h_b1_all_pass"].clone(),
                    "verdicts": verdicts,
                    "recorded_exactly": "outcomes are copied from summary.json; TRADE_OFF_FAILURE is shown as recorded",
                });
                if detail {
                    let mut s = sj.clone();
                    scrub_seeds(&mut s);
                    verdict_full = Some(s);
                }
            }
        }
        _ => {}
    }
    o.insert("verdict_snapshot".into(), verdict_snapshot);
    o.insert("artifacts".into(), json!(artifact_names(&en.dir)));
    o.insert("artifacts_downloadable".into(), json!(false));
    if detail {
        if let Some(m) = &manifest {
            let pick = |k: &str| m.get(k).cloned().unwrap_or(Value::Null);
            o.insert(
                "provenance".into(),
                json!({
                    "manifest_tier": pick("tier"), "name": pick("name"), "run_name": pick("run_name"), "scenario": pick("scenario"),
                    "status": pick("status"), "seed_set": pick("seed_set"), "seed_list_sha256": pick("seed_list_sha256"),
                    "profile": pick("profile"), "label_planned": pick("label_planned"), "label_reasons": pick("label_reasons"),
                    "arms": pick("arms"), "reps": pick("reps"), "phases": pick("phases"),
                    "git_head": m["git"]["head"].clone(),
                }),
            );
        } else {
            o.insert("provenance".into(), unavailable_metric("no readable manifest.json"));
        }
        if let Some(v) = verdict_full {
            o.insert("recorded_result".into(), v);
        }
        if en.kind == "scenario_b" {
            o.insert(
                "interpretation".into(),
                json!({
                    "binding": true,
                    "must_accompany_any_citation": true,
                    "notice_applies_to": "scenario_b/20261030T003932Z (the retained main experiment); the limitations below apply to every Scenario B result",
                    "notice_text": SCENARIO_B_NOTICE,
                    "limitations": sb::limitations(),
                    "label_meaning": sb::LABEL_VALID_MEANING,
                }),
            );
        }
        for (k, v) in extra {
            o.insert(k, v);
        }
    }
    Value::Object(o)
}

async fn evidence_index(State(ctx): State<LabCtx>, headers: HeaderMap, RawQuery(q): RawQuery) -> Response {
    if let Err(r) = authorize(&ctx, &headers, crate::security::Role::Observer) {
        return r;
    }
    let (limit, offset) = match parse_page(q.as_deref()) {
        Ok(v) => v,
        Err(m) => return bad_page(m),
    };
    let all = scan_evidence(&ctx.lab.cfg);
    let prefixes = ctx.lab.scrub_prefixes();
    let items: Vec<Value> = all
        .iter()
        .skip(offset)
        .take(limit)
        .map(|e| {
            let mut v = evidence_summary(e, false);
            scrub_value(&mut v, &prefixes);
            v
        })
        .collect();
    ok_json(
        StatusCode::OK,
        json!({"limit": limit, "offset": offset, "returned": items.len(), "indexed": all.len(), "bounded_to_newest": INDEX_MAX,
               "ordering": "id_descending", "items": items}),
    )
}

async fn evidence_detail(State(ctx): State<LabCtx>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if let Err(r) = authorize(&ctx, &headers, crate::security::Role::Observer) {
        return r;
    }
    match scan_evidence(&ctx.lab.cfg).into_iter().find(|e| e.id == id) {
        Some(e) => {
            let mut v = evidence_summary(&e, true);
            scrub_value(&mut v, &ctx.lab.scrub_prefixes());
            ok_json(StatusCode::OK, v)
        }
        None => not_found(),
    }
}

// ------------------------------------------------------------------------------------------
// POST /runs
// ------------------------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartReq {
    kind: String,
    spec: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct T1Spec {
    name: Option<String>,
    scenarios: Vec<String>,
    families: Vec<String>,
    arms: Vec<String>,
    seed_set: Option<String>,
    n_seeds: usize,
    profile: Option<String>,
    arm_params: Option<String>,
    record_raw: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SbSpec {
    repetitions: Option<usize>,
    arms: Option<Vec<String>>,
    warmup_s: Option<u64>,
    healthy_s: Option<u64>,
    degraded_s: Option<u64>,
    recovery_s: Option<u64>,
    rate_rps: Option<f64>,
    n_keys: Option<usize>,
    static_cap: Option<usize>,
    aimd_min: Option<usize>,
    aimd_max: Option<usize>,
}

fn invalid(msg: &str) -> Response {
    let m: String = msg.chars().take(160).collect();
    err(StatusCode::UNPROCESSABLE_ENTITY, "invalid_spec", &m)
}

enum Prepared {
    T1(RunSpec),
    Sb(Box<sb::ScenarioBConfig>),
}

fn prepare_t1(spec: Value, run_dir: &FsPath) -> Result<(RunSpec, Value), Response> {
    let s: T1Spec = serde_json::from_value(spec).map_err(|e| invalid(&format!("spec: {e}")))?;
    for l in [&s.scenarios, &s.families, &s.arms] {
        if l.len() > MAX_LIST_ITEMS {
            return Err(invalid("a selection list is too long"));
        }
    }
    if s.n_seeds > MAX_SEEDS {
        return Err(invalid("n_seeds exceeds the API limit of 30"));
    }
    let name = s.name.unwrap_or_else(|| "lab-api-run".into());
    if name.is_empty() || name.len() > 64 || !name.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-' | b' ')) {
        return Err(invalid("name must be 1-64 characters of [A-Za-z0-9._ -]"));
    }
    let rs = RunSpec {
        name,
        scenarios: s.scenarios,
        families: s.families,
        arms: s.arms,
        seed_set: s.seed_set.unwrap_or_else(|| "D".into()),
        n_seeds: s.n_seeds,
        profile: s.profile.unwrap_or_else(|| "smoke".into()),
        arm_params: s.arm_params.unwrap_or_else(|| "untuned".into()),
        record_raw: s.record_raw.unwrap_or(false),
        out_dir: run_dir.join("t1").to_string_lossy().to_string(),
        overwrite: false,
    };
    for f in &rs.families {
        if let Some(rest) = f.strip_prefix("W-Z(") {
            match rest.strip_suffix(')').and_then(|x| x.parse::<f64>().ok()) {
                Some(a) if a.is_finite() && a > 0.0 && a <= 10.0 => {}
                _ => return Err(invalid("W-Z alpha must be a finite number in (0, 10]")),
            }
        }
    }
    let cells = runner::cell_count(&rs);
    if cells > MAX_CELLS {
        return Err(invalid("estimated cell count exceeds the API limit of 20000"));
    }
    runner::validate(&rs).map_err(|e| invalid(&e.to_string()))?;
    let echo = json!({
        "name": rs.name, "scenarios": rs.scenarios, "families": rs.families, "arms": rs.arms, "seed_set": rs.seed_set.trim().to_ascii_uppercase(),
        "n_seeds": rs.n_seeds, "profile": rs.profile, "arm_params": rs.arm_params, "record_raw": rs.record_raw, "cells": cells
    });
    Ok((rs, echo))
}

fn prepare_sb(spec: Value, run_dir: &FsPath, exe: PathBuf) -> Result<(Box<sb::ScenarioBConfig>, Value), Response> {
    let s: SbSpec = serde_json::from_value(spec).map_err(|e| invalid(&format!("spec: {e}")))?;
    let reps = s.repetitions.unwrap_or(10);
    if !(1..=MAX_REPETITIONS).contains(&reps) {
        return Err(invalid("repetitions must be in 1..=30"));
    }
    let mut arms = Vec::new();
    match &s.arms {
        None => arms = vec![sb::Arm::Base, sb::Arm::StaticCap, sb::Arm::AimdCap],
        Some(l) => {
            if l.is_empty() || l.len() > 3 {
                return Err(invalid("arms must list 1-3 of BASE, STATIC-CAP, AIMD-CAP"));
            }
            for n in l {
                let a = [sb::Arm::Base, sb::Arm::StaticCap, sb::Arm::AimdCap].into_iter().find(|a| a.name() == n);
                match a {
                    Some(a) if !arms.contains(&a) => arms.push(a),
                    _ => return Err(invalid("arms must be distinct values of BASE, STATIC-CAP, AIMD-CAP")),
                }
            }
        }
    }
    let phase = |v: Option<u64>, d: u64, what: &str| -> Result<u64, Response> {
        let x = v.unwrap_or(d);
        if (1..=60).contains(&x) {
            Ok(x * 1000)
        } else {
            Err(invalid(&format!("{what} must be in 1..=60 seconds")))
        }
    };
    let phases = sb::Phases {
        warmup_ms: phase(s.warmup_s, 8, "warmup_s")?,
        healthy_ms: phase(s.healthy_s, 5, "healthy_s")?,
        degraded_ms: phase(s.degraded_s, 10, "degraded_s")?,
        recovery_ms: phase(s.recovery_s, 8, "recovery_s")?,
    };
    let rate = s.rate_rps.unwrap_or(300.0);
    if !rate.is_finite() || !(1.0..=2000.0).contains(&rate) {
        return Err(invalid("rate_rps must be in 1..=2000"));
    }
    let n_keys = s.n_keys.unwrap_or(5000);
    if !(10..=100_000).contains(&n_keys) {
        return Err(invalid("n_keys must be in 10..=100000"));
    }
    let cap = |v: Option<usize>, d: usize, what: &str| -> Result<usize, Response> {
        let x = v.unwrap_or(d);
        if (1..=1000).contains(&x) {
            Ok(x)
        } else {
            Err(invalid(&format!("{what} must be in 1..=1000")))
        }
    };
    let static_cap = cap(s.static_cap, 24, "static_cap")?;
    let aimd_min = cap(s.aimd_min, 12, "aimd_min")?;
    let aimd_max = cap(s.aimd_max, 32, "aimd_max")?;
    if aimd_min > aimd_max {
        return Err(invalid("aimd_min must be <= aimd_max"));
    }
    let mut c = sb::ScenarioBConfig::with_phases(exe, run_dir.join("out"), phases);
    c.reps = reps;
    c.arms = arms.clone();
    c.workload.rate_rps = rate;
    c.workload.n_keys = n_keys;
    c.static_cap = static_cap;
    c.aimd_min = aimd_min;
    c.aimd_max = aimd_max;
    c.command_line = "lab-api scenario_b".to_string();
    sb::finalize_config(&mut c);
    let echo = json!({
        "repetitions": reps, "arms": arms.iter().map(|a| a.name()).collect::<Vec<_>>(),
        "warmup_s": phases.warmup_ms / 1000, "healthy_s": phases.healthy_ms / 1000, "degraded_s": phases.degraded_ms / 1000, "recovery_s": phases.recovery_ms / 1000,
        "rate_rps": rate, "n_keys": n_keys, "static_cap": static_cap, "aimd_min": aimd_min, "aimd_max": aimd_max
    });
    Ok((Box::new(c), echo))
}

#[allow(clippy::result_large_err)]
fn require_json_content_type(headers: &HeaderMap) -> Result<(), Response> {
    let ok = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_ascii_lowercase().starts_with("application/json"))
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        // Also the anti-CSRF measure: a cross-site form/text POST cannot set this type without a preflight.
        Err(err(StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported_media_type", "Content-Type must be application/json"))
    }
}

async fn runs_start(State(ctx): State<LabCtx>, req: Request<Body>) -> Response {
    let (parts, body) = req.into_parts();
    // 1. authenticate (no body bytes consumed yet, no side effect)
    if let Err(r) = authorize(&ctx, &parts.headers, crate::security::Role::Operator) {
        return r;
    }
    if let Err(r) = require_json_content_type(&parts.headers) {
        return r;
    }
    // 2. bounded, time-limited body read
    if let Some(cl) = parts.headers.get(header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<usize>().ok()) {
        if cl > MAX_BODY_BYTES {
            return err(StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large", "request body exceeds 16 KiB");
        }
    }
    let bytes: Bytes = match tokio::time::timeout(BODY_READ_TIMEOUT, to_bytes(body, MAX_BODY_BYTES)).await {
        Err(_) => return err(StatusCode::REQUEST_TIMEOUT, "request_timeout", "the request body was not received in time"),
        Ok(Err(_)) => return err(StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large", "request body exceeds 16 KiB"),
        Ok(Ok(b)) => b,
    };
    // 3. parse: syntax -> 400, schema/unknown field -> 422
    let sr: StartReq = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            return if e.is_syntax() || e.is_eof() {
                err(StatusCode::BAD_REQUEST, "bad_request", "request body is not valid JSON")
            } else {
                invalid(&format!("request: {e}"))
            }
        }
    };
    let run_id = new_run_id();
    let run_dir = ctx.lab.cfg.runs_dir().join(&run_id);
    let (prepared, echo, kind) = match sr.kind.as_str() {
        "t1_matrix" => match prepare_t1(sr.spec, &run_dir) {
            Ok((rs, echo)) => (Prepared::T1(rs), echo, "t1_matrix"),
            Err(r) => return r,
        },
        "scenario_b" => {
            let exe = match ctx.lab.gateway_status() {
                Ok(p) => p,
                Err(reason) => return invalid(&format!("scenario_b is unavailable: {reason}")),
            };
            match prepare_sb(sr.spec, &run_dir, exe) {
                Ok((c, echo)) => (Prepared::Sb(c), echo, "scenario_b"),
                Err(r) => return r,
            }
        }
        _ => return invalid("kind must be t1_matrix or scenario_b"),
    };

    // 4. one active run at a time
    let tier = if kind == "t1_matrix" { "SIMULATED" } else { sb::TIER };
    let handle = {
        let mut a = ctx.lab.active.lock().unwrap_or_else(|e| e.into_inner());
        if a.is_some() {
            return err(StatusCode::CONFLICT, "run_active", "a run is already active");
        }
        let v = json!({
            "run_id": run_id, "kind": kind, "state": "queued", "tier": tier, "spec": echo,
            "created_utc": utc_now_iso(), "started_utc": null, "ended_utc": null, "exit_code": null,
            "progress": {"done": 0, "total": null}, "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        });
        match RunHandle::create(run_dir.clone(), run_id.clone(), v, Arc::new(AtomicBool::new(false)), ctx.lab.scrub_prefixes()) {
            Ok(h) => {
                *a = Some(h.clone());
                h
            }
            Err(_) => return err(StatusCode::INTERNAL_SERVER_ERROR, "internal", "internal error"),
        }
    };

    let svc = ctx.lab.clone();
    let h2 = handle.clone();
    let name = format!("lab-run-{run_id}");
    let spawn = std::thread::Builder::new().name(name).spawn(move || {
        let _guard = ActiveGuard(&svc, h2.id.clone());
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| worker(prepared, &h2, &svc)));
        if r.is_err() {
            // a panic in the worker's own bookkeeping: never leave the run non-terminal
            h2.finish(&svc, "failed", Some(1), None, Some(("worker_panic", "the experiment worker terminated abnormally")), None);
        }
    });
    if spawn.is_err() {
        handle.finish(&ctx.lab, "failed", Some(1), None, Some(("spawn", "could not start the worker")), None);
        return err(StatusCode::INTERNAL_SERVER_ERROR, "internal", "internal error");
    }
    ok_json(
        StatusCode::ACCEPTED,
        json!({"run_id": run_id, "state": "queued", "kind": kind, "tier": tier,
               "links": {"self": format!("/api/v1/lab/runs/{run_id}"), "events": format!("/api/v1/lab/runs/{run_id}/events"),
                         "cancel": format!("/api/v1/lab/runs/{run_id}/cancel"), "artifacts": format!("/api/v1/lab/runs/{run_id}/artifacts")}}),
    )
}

// ------------------------------------------------------------------------------------------
// Worker
// ------------------------------------------------------------------------------------------

struct CancelMarker;

fn worker(prepared: Prepared, h: &Arc<RunHandle>, svc: &Arc<LabService>) {
    h.begin_running();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match prepared {
        Prepared::T1(spec) => run_t1(spec, h, svc),
        Prepared::Sb(cfg) => run_sb(*cfg, h, svc),
    }));
    if let Err(p) = caught {
        if p.downcast_ref::<CancelMarker>().is_some() {
            h.finish(svc, "cancelled", Some(130), None, None, None);
        } else {
            h.finish(svc, "failed", Some(1), None, Some(("runner_panic", "the experiment runner terminated abnormally")), None);
        }
    }
}

fn kind_of(h: &RunHandle) -> String {
    h.inner.lock().unwrap_or_else(|e| e.into_inner()).v["kind"].as_str().unwrap_or("").to_string()
}

fn run_t1(spec: RunSpec, h: &Arc<RunHandle>, svc: &Arc<LabService>) {
    let mut last = Instant::now() - Duration::from_secs(1);
    let h2 = h.clone();
    let res = runner::execute(&spec, &mut |done, total| {
        if h2.cancel.load(Ordering::SeqCst) {
            // cooperative cancellation at a cell boundary; unwinds out of the runner, caught in `worker`
            std::panic::resume_unwind(Box::new(CancelMarker));
        }
        if done == 1 || done == total || last.elapsed() >= Duration::from_millis(200) {
            last = Instant::now();
            h2.progress(json!({"done": done, "total": total, "tier": "SIMULATED"}), None);
        }
    });
    match res {
        Ok(rr) => {
            let sha = sha256_file(&rr.out_dir.join("cells.csv")).ok();
            let ix = svc.build_artifact_index(&h.dir, "t1_matrix");
            h.finish(
                svc,
                "completed",
                Some(0),
                Some(json!({"tier": "SIMULATED", "cells": rr.cells.len(), "cells_csv_sha256": sha,
                            "note": "virtual-time simulation on the real cache/executor; not a measurement"})),
                None,
                Some(ix),
            );
        }
        Err(e) => {
            let msg = match e {
                runner::LabError::Io(_) => "the experiment could not write its output".to_string(),
                other => other.to_string().chars().take(160).collect(),
            };
            h.finish(svc, "failed", Some(1), None, Some(("run_failed", &msg)), None);
        }
    }
}

fn run_sb(cfg: sb::ScenarioBConfig, h: &Arc<RunHandle>, svc: &Arc<LabService>) {
    let rt = match tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() {
        Ok(r) => r,
        Err(_) => {
            h.finish(svc, "failed", Some(1), None, Some(("runtime", "could not create the experiment runtime")), None);
            return;
        }
    };
    let h2 = h.clone();
    let progress: sb::ProgressFn = Arc::new(move |p: sb::Progress| {
        h2.progress(
            json!({"done": p.done, "total": p.total, "arm": p.arm, "rep": p.rep + 1, "phase": p.phase, "gateway_pid": p.gateway_pid, "tier": sb::TIER}),
            Some(p.gateway_pid),
        );
    });
    let out = rt.block_on(sb::run_experiment(&cfg, h.cancel.clone(), progress));
    drop(rt);
    let kind = kind_of(h);
    match out {
        Ok(o) if o.cancelled => h.finish(svc, "cancelled", Some(130), None, None, Some(svc.build_artifact_index(&h.dir, &kind))),
        Ok(o) => {
            // U-01: null verdicts (incomplete summary) must stay null, never become an empty object.
            let verdicts: Value = match o.summary["verdicts"].as_object() {
                Some(m) => Value::Object(m.iter().map(|(k, v)| (k.clone(), json!({"outcome": v["outcome"], "reasons": v["reasons"]}))).collect()),
                None => Value::Null,
            };
            let ix = svc.build_artifact_index(&h.dir, &kind);
            h.finish(
                svc,
                "completed",
                Some(0),
                Some(json!({"tier": sb::TIER, "label": o.label, "label_reasons": o.summary["label_reasons"], "verdicts": verdicts,
                            "label_meaning": sb::LABEL_VALID_MEANING, "limitations": sb::limitations(),
                            "interpretation_notice_ref": "evidence payloads of kind scenario_b carry the binding notice text"})),
                None,
                Some(ix),
            );
        }
        Err(_) => h.finish(svc, "failed", Some(1), None, Some(("scenario_failed", "the experiment failed; see manifest.json for the recorded status")), Some(svc.build_artifact_index(&h.dir, &kind))),
    }
}

// ------------------------------------------------------------------------------------------
// GET /runs, GET /runs/{id}, cancel
// ------------------------------------------------------------------------------------------

fn run_links(id: &str) -> Value {
    json!({"events": format!("/api/v1/lab/runs/{id}/events"), "artifacts": format!("/api/v1/lab/runs/{id}/artifacts"), "cancel": format!("/api/v1/lab/runs/{id}/cancel")})
}

async fn runs_list(State(ctx): State<LabCtx>, headers: HeaderMap, RawQuery(q): RawQuery) -> Response {
    if let Err(r) = authorize(&ctx, &headers, crate::security::Role::Observer) {
        return r;
    }
    let (limit, offset) = match parse_page(q.as_deref()) {
        Ok(v) => v,
        Err(m) => return bad_page(m),
    };
    let ids = ctx.lab.list_run_ids();
    let items: Vec<Value> = ids
        .iter()
        .skip(offset)
        .take(limit)
        .filter_map(|id| {
            let dir = ctx.lab.run_dir(id)?;
            let v = read_json_capped(&dir.join("run.json"))?;
            Some(json!({"run_id": id, "kind": v["kind"], "state": v["state"], "tier": v["tier"], "created_utc": v["created_utc"],
                        "started_utc": v["started_utc"], "ended_utc": v["ended_utc"], "progress": v["progress"]}))
        })
        .collect();
    ok_json(
        StatusCode::OK,
        json!({"limit": limit, "offset": offset, "returned": items.len(), "indexed": ids.len(), "bounded_to_newest": INDEX_MAX,
               "ordering": "newest_first", "items": items}),
    )
}

async fn run_get(State(ctx): State<LabCtx>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if let Err(r) = authorize(&ctx, &headers, crate::security::Role::Observer) {
        return r;
    }
    let Some(dir) = ctx.lab.run_dir(&id) else { return not_found() };
    let Some(mut v) = read_json_capped(&dir.join("run.json")) else { return not_found() };
    v["links"] = run_links(&id);
    let (svc1, id1) = (ctx.lab.clone(), id.clone());
    v["active"] = json!(blk(move || svc1.handle_for(&id1).is_some()).await);
    if v["kind"] == "scenario_b" {
        v["interpretation"] = json!({
            "label_meaning": sb::LABEL_VALID_MEANING,
            "limitations": sb::limitations(),
            "notice": "see GET /api/v1/lab/evidence/scenario_b.* for the binding interpretation notice",
        });
    }
    if matches!(v["state"].as_str(), Some("completed")) {
        let f = if v["kind"] == "scenario_b" { "summary.json" } else { "cells.csv" };
        v["summary_link"] = json!(format!("/api/v1/lab/runs/{id}/artifacts/{f}"));
    }
    scrub_value(&mut v, &ctx.lab.scrub_prefixes());
    ok_json(StatusCode::OK, v)
}

async fn run_cancel(State(ctx): State<LabCtx>, Path(id): Path<String>, req: Request<Body>) -> Response {
    let (parts, _body) = req.into_parts();
    if let Err(r) = authorize(&ctx, &parts.headers, crate::security::Role::Operator) {
        return r;
    }
    if let Err(r) = require_json_content_type(&parts.headers) {
        return r;
    }
    let Some(dir) = ctx.lab.run_dir(&id) else { return not_found() };
    // the active run: ONE handle, so state transitions and events share one lock
    let (svc2, id2) = (ctx.lab.clone(), id.clone());
    let cancelled = blk(move || {
        svc2.handle_for(&id2).map(|h| {
            h.cancel.store(true, Ordering::SeqCst);
            let changed = h.request_cancel();
            (changed, h.state())
        })
    })
    .await;
    if let Some((changed, st)) = cancelled {
        let code = if is_terminal(&st) { StatusCode::OK } else { StatusCode::ACCEPTED };
        return ok_json(code, json!({"run_id": id, "state": st, "changed": changed}));
    }
    let Some(v) = read_json_capped(&dir.join("run.json")) else { return not_found() };
    ok_json(StatusCode::OK, json!({"run_id": id, "state": v["state"], "changed": false}))
}

// ------------------------------------------------------------------------------------------
// SSE
// ------------------------------------------------------------------------------------------

struct Tail {
    path: PathBuf,
    run_json: PathBuf,
    handle: Option<Arc<RunHandle>>,
    rx: Option<tokio::sync::watch::Receiver<u64>>,
    exp: Option<u64>,
    offset: u64,
    after: u64,
    pending: VecDeque<(u64, String, String)>,
    done: bool,
    saw_done_event: bool,
    _permit: SsePermit,
}

impl Tail {
    fn read_new(&mut self) {
        let Ok(mut f) = std::fs::File::open(&self.path) else { return };
        if f.seek(SeekFrom::Start(self.offset)).is_err() {
            return;
        }
        let mut buf = Vec::new();
        if f.take(1024 * 1024).read_to_end(&mut buf).is_err() {
            return;
        }
        let mut consumed = 0usize;
        while let Some(nl) = buf[consumed..].iter().position(|b| *b == b'\n') {
            let line = &buf[consumed..consumed + nl];
            consumed += nl + 1;
            if line.len() > MAX_EVENT_LINE {
                self.done = true;
                return;
            }
            if let Ok(v) = serde_json::from_slice::<Value>(line) {
                if let (Some(id), Some(ev)) = (v["id"].as_u64(), v["event"].as_str()) {
                    if id > self.after {
                        if ev == "done" {
                            self.saw_done_event = true;
                        }
                        self.pending.push_back((id, ev.to_string(), v["data"].to_string()));
                        self.after = id;
                    }
                }
            }
        }
        if buf.len() - consumed > MAX_EVENT_LINE {
            self.done = true; // an unterminated line longer than any event this server writes
        }
        self.offset += consumed as u64;
    }

    fn expired(&self) -> bool {
        self.exp.map(|e| now_unix() >= e).unwrap_or(false)
    }
}

async fn run_events(State(ctx): State<LabCtx>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    let exp = match authorize(&ctx, &headers, crate::security::Role::Observer) {
        Ok(e) => e,
        Err(r) => return r,
    };
    let Some(dir) = ctx.lab.run_dir(&id) else { return not_found() };
    let after = match headers.get("last-event-id") {
        None => 0,
        Some(v) => match v.to_str().ok().and_then(|s| s.trim().parse::<u64>().ok()) {
            Some(n) => n,
            None => return bad_page("Last-Event-ID must be a non-negative integer"),
        },
    };
    if ctx.lab.sse_open.fetch_add(1, Ordering::SeqCst) >= MAX_SSE_STREAMS {
        ctx.lab.sse_open.fetch_sub(1, Ordering::SeqCst);
        return err(StatusCode::TOO_MANY_REQUESTS, "too_many_streams", "too many open event streams");
    }
    let permit = SsePermit(ctx.lab.clone());
    let (svc3, id3) = (ctx.lab.clone(), id.clone());
    let handle = blk(move || svc3.handle_for(&id3)).await;
    let tail = Tail {
        rx: handle.as_ref().map(|h| h.tx.subscribe()),
        handle,
        path: dir.join("events.jsonl"),
        run_json: dir.join("run.json"),
        exp,
        offset: 0,
        after,
        pending: VecDeque::new(),
        done: false,
        saw_done_event: false,
        _permit: permit,
    };
    let stream = futures_util::stream::unfold(tail, |mut t| async move {
        loop {
            if t.expired() {
                return None; // the token that opened this stream has expired
            }
            if let Some((id, ev, data)) = t.pending.pop_front() {
                let e = Event::default().id(id.to_string()).event(ev).data(data);
                return Some((Ok::<Event, std::convert::Infallible>(e), t));
            }
            if t.done || t.saw_done_event {
                return None;
            }
            // capture the terminal flag BEFORE reading: events are written before run.json turns terminal
            let state = read_json_capped(&t.run_json).and_then(|v| v["state"].as_str().map(|s| s.to_string())).unwrap_or_default();
            let gone = !is_terminal(&state) && t.handle.as_ref().map(|h| h.terminal.load(Ordering::Acquire)).unwrap_or(true);
            t.read_new();
            if t.pending.is_empty() && !t.done {
                if is_terminal(&state) || gone {
                    t.done = true;
                } else if let Some(rx) = t.rx.as_mut() {
                    // woken by the writer; the 2 s timeout bounds expiry checks and covers missed edges
                    let _ = tokio::time::timeout(Duration::from_secs(2), rx.changed()).await;
                } else {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });
    let mut resp = Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(HEARTBEAT_SECS)).text("heartbeat"))
        .into_response();
    resp.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    resp.headers_mut().insert("x-accel-buffering", HeaderValue::from_static("no"));
    resp
}

// ------------------------------------------------------------------------------------------
// Artifacts
// ------------------------------------------------------------------------------------------

/// (sub-directory, file name, content type) of every downloadable artifact of a run kind.
fn artifact_allowlist(kind: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    let mut v = vec![("", "run.json", "application/json"), ("", "events.jsonl", "application/x-ndjson")];
    match kind {
        "t1_matrix" => {
            v.push(("t1", "cells.csv", "text/csv"));
            v.push(("t1", "manifest.json", "application/json"));
            v.push(("t1", "requests.csv", "text/csv"));
            v.push(("t1", "fetches.csv", "text/csv"));
        }
        "scenario_b" => {
            v.push(("out", "summary.json", "application/json"));
            v.push(("out", "manifest.json", "application/json"));
        }
        _ => {}
    }
    v
}

fn regular_file(p: &FsPath) -> Option<u64> {
    std::fs::symlink_metadata(p).ok().filter(|m| m.is_file() && !is_reparse(m)).map(|m| m.len())
}

/// Path of an allowlisted artifact; every directory level (`t1/`, `out/`) must be a real directory.
fn artifact_path(dir: &FsPath, sub: &str, name: &str) -> Option<PathBuf> {
    if sub.is_empty() {
        return Some(dir.join(name));
    }
    let d = dir.join(sub);
    if plain_dir(&d) {
        Some(d.join(name))
    } else {
        None
    }
}

async fn artifacts_list(State(ctx): State<LabCtx>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if let Err(r) = authorize(&ctx, &headers, crate::security::Role::Observer) {
        return r;
    }
    let Some(dir) = ctx.lab.run_dir(&id) else { return not_found() };
    let Some(v) = read_json_capped(&dir.join("run.json")) else { return not_found() };
    let kind = v["kind"].as_str().unwrap_or("");
    let mut items = Vec::new();
    for (sub, name, ctype) in artifact_allowlist(kind) {
        let Some(p) = artifact_path(&dir, sub, name) else { continue };
        let Some(len) = regular_file(&p) else { continue };
        // hashes were computed once when the run ended (run.json `artifact_index`); listing is stat-only
        let live = name == "run.json" || name == "events.jsonl";
        let sha = match v["artifact_index"][name].as_object() {
            Some(e) if !live && e.get("bytes").and_then(|b| b.as_u64()) == Some(len) => e["sha256"].clone(),
            _ if live => unavailable_metric("live file; not hashed"),
            _ => unavailable_metric("no hash recorded for the current file (run not finished, or the file changed or exceeds the 64 MiB cap)"),
        };
        items.push(json!({"name": name, "bytes": len, "sha256": sha, "content_type": ctype, "downloadable": len <= MAX_ARTIFACT_BYTES,
                          "sha256_of": if name.ends_with(".json") && !live { "served bytes (server paths redacted)" } else { "file bytes" },
                          "tier": if live { Value::Null } else { v["tier"].clone() }}));
    }
    ok_json(StatusCode::OK, json!({"run_id": id, "state": v["state"], "artifacts": items}))
}

async fn artifact_get(State(ctx): State<LabCtx>, headers: HeaderMap, Path((id, name)): Path<(String, String)>) -> Response {
    if let Err(r) = authorize(&ctx, &headers, crate::security::Role::Observer) {
        return r;
    }
    let Some(dir) = ctx.lab.run_dir(&id) else { return not_found() };
    let Some(v) = read_json_capped(&dir.join("run.json")) else { return not_found() };
    // Equality lookup against a fixed allowlist: the client string is never joined to a path.
    let Some((sub, fname, ctype)) = artifact_allowlist(v["kind"].as_str().unwrap_or("")).into_iter().find(|(_, n, _)| *n == name) else {
        return not_found();
    };
    let Some(p) = artifact_path(&dir, sub, fname) else { return not_found() };
    let Some((std_file, len)) = open_regular(&p) else { return not_found() };
    if len > MAX_ARTIFACT_BYTES {
        return err(StatusCode::PAYLOAD_TOO_LARGE, "artifact_too_large", "artifact exceeds the 64 MiB download cap");
    }
    let disp = (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{fname}\""));
    let common = [(header::CONTENT_TYPE, ctype.to_string()), disp, (header::CACHE_CONTROL, "no-store".to_string()), (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string())];
    if fname.ends_with(".json") {
        // JSON artifacts are served with server filesystem paths redacted (<= 8 MiB, parsed once)
        let prefixes = ctx.lab.scrub_prefixes();
        let p2 = p.clone();
        let bytes = tokio::task::spawn_blocking(move || served_json_bytes(&p2, &prefixes)).await.ok().flatten();
        return match bytes {
            Some(b) => (StatusCode::OK, common, b).into_response(),
            None => err(StatusCode::PAYLOAD_TOO_LARGE, "artifact_unreadable", "artifact is too large or not valid JSON"),
        };
    }
    // everything else is streamed in 64 KiB chunks, never held in memory
    use tokio::io::AsyncReadExt;
    let file = tokio::fs::File::from_std(std_file);
    let stream = futures_util::stream::unfold((Some(file), len), |(f, remaining)| async move {
        let mut f = f?;
        if remaining == 0 {
            return None;
        }
        let mut buf = vec![0u8; remaining.min(64 * 1024) as usize];
        match f.read(&mut buf).await {
            Ok(0) => None,
            Ok(n) => {
                buf.truncate(n);
                Some((Ok::<Bytes, std::io::Error>(Bytes::from(buf)), (Some(f), remaining - n as u64)))
            }
            Err(e) => Some((Err(e), (None, 0))),
        }
    });
    let mut resp = (StatusCode::OK, common, Body::from_stream(stream)).into_response();
    resp.headers_mut().insert(header::CONTENT_LENGTH, HeaderValue::from(len));
    resp
}

/// This process and its ancestors (best effort; Windows only, otherwise just this process).
pub fn process_ancestors() -> Vec<u32> {
    #[cfg(windows)]
    {
        win::self_and_ancestors()
    }
    #[cfg(not(windows))]
    {
        vec![std::process::id()]
    }
}
