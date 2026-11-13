//! Deterministic PRNG stream splitting and O(1) discrete sampling (M4-01 / M4-02).
//!
//! Features:
//! - SplitMix64 generator with guaranteed period 2^64.
//! - Master seed splitting into named, orthogonal streams (`arrivals`, `keys`, `sizes`, `origin`, `faults`).
//! - Disjoint seed sets: Tuning (T), Development-Evaluation (D), and Sealed (S; hash-committed, derived at run time from a secret salt - never stored in the repository).
//! - Vose's Alias Method for O(1) sampling from arbitrary discrete distributions (Zipf).

use std::collections::HashSet;

/// SplitMix64 deterministic pseudorandom number generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamRng {
    state: u64,
}

impl StreamRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    pub fn next_f64(&mut self) -> f64 {
        // 53 bits of precision in [0.0, 1.0)
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn next_range_u64(&mut self, min: u64, max: u64) -> u64 {
        assert!(max >= min);
        if max == min {
            return min;
        }
        let range = max - min + 1;
        min + (self.next_u64() % range)
    }

    /// Samples an exponential random variable with parameter lambda (mean 1.0 / lambda).
    pub fn next_exponential(&mut self, lambda: f64) -> f64 {
        assert!(lambda > 0.0);
        let u = 1.0 - self.next_f64().max(1e-15);
        -u.ln() / lambda
    }
}

/// Master seed splitter deriving named orthogonal random streams.
#[derive(Debug, Clone)]
pub struct MasterRng {
    master_seed: u64,
}

impl MasterRng {
    pub fn new(master_seed: u64) -> Self {
        Self { master_seed }
    }

    /// Derives an independent stream for a named subsystem.
    pub fn stream(&self, stream_name: &str) -> StreamRng {
        // Deterministic hash combining master seed and stream name
        let mut h = self.master_seed.wrapping_add(0x517cc1b727220a95);
        for &b in stream_name.as_bytes() {
            h = h.wrapping_mul(0x9e3779b97f4a7c15).wrapping_add(b as u64);
        }
        // Warm up SplitMix64 state
        let mut rng = StreamRng::new(h);
        let _ = rng.next_u64();
        let _ = rng.next_u64();
        rng
    }
}

// ---------------------------------------------------------------------------
// Disjoint Seed Sets: Tuning (T), Development (D), and Sealed (S)
// ---------------------------------------------------------------------------

pub struct SeedSets;

/// SHA-256 commitment to the secret 32-byte salt from which the sealed seed set S is derived
/// (preregistration_m6_v2.md section 6). The salt itself is NOT in the repository.
///
/// Protocol v2.1 (amendment of 2026-10-30): this commitment REPLACES the v2 commitment
/// `bc7c9683fb46b1e4d8f3b2c01612668c9591427402e49c576220bba3724f0e75`, whose salt was generated inside
/// an earlier agent session and is retired as exposed (it was never used to derive an evaluation seed).
/// The v2.1 salt was generated from the operating-system CSPRNG at 2026-10-29T22:12:07Z, never printed or
/// logged, and stored outside the repository; same-user processes can still read that file.
pub const SEALED_SALT_COMMITMENT_SHA256: &str =
    "60167cd235911e0c10377b68fc725c6c6a892f41480707482f43ec9578f667cc";

/// Domain separation string for sealed seed derivation (v2.1; differs from v2 so that no seed derived
/// under the retired salt can coincide with a v2.1 seed).
pub const SEALED_DOMAIN: &str = "phantomcache-m6-sealed-v2.1";

/// Protocol version recorded in the freeze record and checked by `verify_freeze`.
/// 2.2 (amendment of 2026-10-30): same salt commitment and derivation domain as 2.1; see preregistration_m6_v2_2_amendment.md.
pub const PROTOCOL_VERSION: &str = "2.2";

/// Number of sealed seeds.
pub const SEALED_COUNT: usize = 30;

/// The legacy plaintext "sealed" range of preregistration v1 (30001..=30030) is burned: it was
/// public in the repository and executed against synthetic data. It must never be used again
/// and is rejected by every leak check.
pub const BURNED_LEGACY_SEALED_RANGE: std::ops::RangeInclusive<u64> = 30_001..=30_030;

pub fn sha256_hex(data: &[u8]) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, data);
    hex::encode(digest.as_ref())
}

pub fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

impl SeedSets {
    /// 60 Tuning seeds (T) used exclusively for baseline & candidate hyperparameter tuning.
    pub fn tuning_seeds() -> Vec<u64> {
        (10_001..=10_060).collect()
    }

    /// 30 Development-Evaluation seeds (D) used for integration tests and development checks.
    pub fn dev_eval_seeds() -> Vec<u64> {
        (20_001..=20_030).collect()
    }

    /// Derives the sealed seed set from a salt that hashes to `expected_commitment`.
    /// seed_i = big-endian u64 from the first 8 bytes of SHA256(domain || 0x00 || salt || i_be32),
    /// with the top bit cleared. Fails if the salt does not match the commitment, or if a derived
    /// seed collides with T, D or the burned legacy range.
    pub fn derive_sealed_seeds(salt: &[u8], expected_commitment: &str) -> Result<Vec<u64>, String> {
        let got = sha256_hex(salt);
        if got != expected_commitment {
            return Err("salt does not match the committed SHA-256 (refusing to derive sealed seeds)".to_string());
        }
        let mut out = Vec::with_capacity(SEALED_COUNT);
        for i in 0..SEALED_COUNT as u32 {
            let mut buf = Vec::new();
            buf.extend_from_slice(SEALED_DOMAIN.as_bytes());
            buf.push(0);
            buf.extend_from_slice(salt);
            buf.extend_from_slice(&i.to_be_bytes());
            let d = ring::digest::digest(&ring::digest::SHA256, &buf);
            let mut b8 = [0u8; 8];
            b8.copy_from_slice(&d.as_ref()[..8]);
            out.push(u64::from_be_bytes(b8) & 0x7FFF_FFFF_FFFF_FFFF);
        }
        let mut uniq = out.clone();
        uniq.sort_unstable();
        uniq.dedup();
        if uniq.len() != SEALED_COUNT {
            return Err("derived sealed seeds are not distinct".to_string());
        }
        let other: HashSet<u64> = Self::tuning_seeds().into_iter().chain(Self::dev_eval_seeds()).collect();
        if out.iter().any(|s| other.contains(s) || BURNED_LEGACY_SEALED_RANGE.contains(s)) {
            return Err("derived sealed seed collides with T/D/burned range".to_string());
        }
        Ok(out)
    }

    /// Run-time access to the real sealed seeds: reads the hex salt from the environment variable
    /// `PHANTOM_SEALED_SALT_HEX` (never stored in the repository) and checks it against the
    /// committed hash. Returns Err if the variable is unset or wrong.
    pub fn sealed_seeds_from_env() -> Result<Vec<u64>, String> {
        let hex_salt = std::env::var("PHANTOM_SEALED_SALT_HEX")
            .map_err(|_| "PHANTOM_SEALED_SALT_HEX is not set".to_string())?;
        let salt = hex_decode(&hex_salt).ok_or_else(|| "PHANTOM_SEALED_SALT_HEX is not valid hex".to_string())?;
        Self::derive_sealed_seeds(&salt, SEALED_SALT_COMMITMENT_SHA256)
    }

    /// Synthetic salt used ONLY by unit tests of the derivation mechanism. Its commitment is
    /// computed on the fly; it has no relation to the real sealed set.
    pub fn derive_with_test_salt(test_salt: &[u8]) -> Vec<u64> {
        Self::derive_sealed_seeds(test_salt, &sha256_hex(test_salt)).expect("test salt derivation")
    }

    /// Seeds that must never appear in tuning / development logs: the burned legacy range, plus
    /// the real sealed seeds when the salt is available in the environment.
    pub fn forbidden_sealed_set() -> HashSet<u64> {
        let mut set: HashSet<u64> = BURNED_LEGACY_SEALED_RANGE.collect();
        if let Ok(real) = Self::sealed_seeds_from_env() {
            set.extend(real);
        }
        set
    }

    /// Asserts that a given set of observed seeds contains zero overlap with the forbidden
    /// sealed set (burned range always; real sealed seeds when the salt is available).
    pub fn verify_no_sealed_leak(observed_seeds: &[u64]) -> Result<(), u64> {
        let sealed = Self::forbidden_sealed_set();
        for &s in observed_seeds {
            if sealed.contains(&s) {
                return Err(s); // Leak detected!
            }
        }
        Ok(())
    }
}

// Minimal hex encoding helper without external crate dependencies
mod hex {
    pub fn encode(data: &[u8]) -> String {
        let mut s = String::with_capacity(data.len() * 2);
        for &b in data {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}

// ---------------------------------------------------------------------------
// Vose's Alias Method: O(1) Discrete Sampler
// ---------------------------------------------------------------------------

/// Vose's Alias Method for O(1) discrete probability sampling.
/// Precomputed once at startup in O(N) time and memory, allowing instant sampling
/// without O(N) linear scans on every request.
#[derive(Debug, Clone)]
pub struct AliasSampler {
    prob: Vec<f64>,
    alias: Vec<usize>,
    n: usize,
}

impl AliasSampler {
    /// Builds an AliasSampler from an unnormalized slice of positive weights.
    pub fn new(weights: &[f64]) -> Self {
        assert!(!weights.is_empty(), "Weights must not be empty");
        let n = weights.len();
        let sum: f64 = weights.iter().sum();
        assert!(sum > 0.0, "Weight sum must be positive");

        let mut prob = vec![0.0; n];
        let mut alias = vec![0; n];

        let mut small = Vec::with_capacity(n);
        let mut large = Vec::with_capacity(n);

        // Scaled probabilities: p_i = weights[i] * n / sum
        let mut scaled: Vec<f64> = weights.iter().map(|&w| (w * n as f64) / sum).collect();

        for (i, &p) in scaled.iter().enumerate() {
            if p < 1.0 {
                small.push(i);
            } else {
                large.push(i);
            }
        }

        while let (Some(l), Some(g)) = (small.pop(), large.pop()) {
            prob[l] = scaled[l];
            alias[l] = g;

            scaled[g] = (scaled[g] + scaled[l]) - 1.0;
            if scaled[g] < 1.0 {
                small.push(g);
            } else {
                large.push(g);
            }
        }

        while let Some(g) = large.pop() {
            prob[g] = 1.0;
        }
        while let Some(l) = small.pop() {
            prob[l] = 1.0;
        }

        Self { prob, alias, n }
    }

    /// Precomputes an AliasSampler for a Zipf distribution over `n` items with parameter `alpha`.
    /// P(rank = k) ~ 1 / k^alpha for k in 1..=n.
    pub fn new_zipf(n: usize, alpha: f64) -> Self {
        let weights: Vec<f64> = (1..=n).map(|k| 1.0 / (k as f64).powf(alpha)).collect();
        Self::new(&weights)
    }

    /// Samples an index in `0..n` in strictly O(1) time.
    pub fn sample(&self, rng: &mut StreamRng) -> usize {
        let i = rng.next_range_u64(0, (self.n - 1) as u64) as usize;
        let coin = rng.next_f64();
        if coin < self.prob[i] {
            i
        } else {
            self.alias[i]
        }
    }
}

