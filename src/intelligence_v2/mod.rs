//! PIE v2 core library (lane L3): shadow-mode Phantom Intelligence.
//!
//! Design: `docs/research/pie_architecture_design_b3.md`. API contract: `docs/lanes/CONTRACT-pie-api-v1.md`.
//!
//! Hard rules enforced here (not promised):
//! - Only `Off`, `Observe`, `Advisory` exist as a type (`Mode`); `canary` / `active` are rejected by `parse_mode`.
//! - The library has no write path to the cache, to any cap or to any epoch: it only reads events and emits
//!   advisory records. Nothing is ever applied.
//! - No dependency on the HTTP layer. The only hot-path call is `EventSink::try_record` (one atomic load when
//!   off, a non-blocking `try_send` otherwise).
//! - No benefit claim: M6 H1 stays NOT_SUPPORTED. Shadow estimates are not verified on the serving path.
//! - Determinism: no wall clock, no thread-id and no iteration over hash maps affect any output; the caller
//!   supplies `now_ms`.

use serde::Serialize;

pub mod api;
pub mod arbiter;
pub mod engine;
pub mod insight;
pub mod policies;
pub mod registry;
pub mod signals;

#[cfg(test)]
mod tests;

pub use engine::{Pie, PieConfig};

pub const SCHEMA: &str = "pie-api-v1";
pub const REGISTRY_VERSION: &str = "1.0.0";

/// Runtime mode. There is intentionally no variant above `Advisory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Off = 0,
    Observe = 1,
    Advisory = 2,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Observe => "observe",
            Mode::Advisory => "advisory",
        }
    }
    pub(crate) fn from_u8(v: u8) -> Mode {
        match v {
            1 => Mode::Observe,
            2 => Mode::Advisory,
            _ => Mode::Off,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeError {
    /// `canary` / `active` (or anything else above advisory) is forbidden in this programme (D-017).
    ForbiddenMode(String),
    Unknown(String),
}

impl std::fmt::Display for ModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModeError::ForbiddenMode(s) => write!(
                f,
                "mode '{s}' is forbidden: only off, observe, advisory run"
            ),
            ModeError::Unknown(s) => write!(f, "unknown mode '{s}'"),
        }
    }
}

/// Parses a mode name. `canary` and `active` are rejected, never mapped to a lower mode.
pub fn parse_mode(s: &str) -> Result<Mode, ModeError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "off" => Ok(Mode::Off),
        "observe" => Ok(Mode::Observe),
        "advisory" => Ok(Mode::Advisory),
        "canary" | "active" => Err(ModeError::ForbiddenMode(s.to_string())),
        _ => Err(ModeError::Unknown(s.to_string())),
    }
}

/// Provenance label of a value (CLAUDE.md section 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Src {
    Measured,
    Derived,
    Simulated,
    Unavailable,
}

impl Src {
    pub fn as_str(self) -> &'static str {
        match self {
            Src::Measured => "measured",
            Src::Derived => "derived",
            Src::Simulated => "simulated",
            Src::Unavailable => "unavailable",
        }
    }
    /// Provenance gate: only measured or derived-from-measured inputs may drive a recommendation.
    pub fn passes_gate(self) -> bool {
        matches!(self, Src::Measured | Src::Derived)
    }
}

/// A feature value: number or flag.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(untagged)]
pub enum FVal {
    F(f64),
    B(bool),
}

impl FVal {
    pub fn as_f64(self) -> f64 {
        match self {
            FVal::F(x) => x,
            FVal::B(b) => {
                if b {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

/// Deterministic key hash for callers (FNV-1a 64 plus a SplitMix finalizer). Same bytes, same hash, on every host.
pub fn hash_key(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    mix64(h)
}

/// SplitMix64 finalizer, used so that spatial sampling (`hash % R == 0`) is uniform even for weak input hashes.
pub fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    OutOfBounds {
        name: String,
        value: f64,
        min: f64,
        max: f64,
    },
    UnknownParameter(String),
    NotFinite(String),
    MemoryBudget {
        needed: usize,
        budget: usize,
    },
    Registry(String),
    Invalid(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
