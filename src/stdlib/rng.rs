//! Tiny xorshift64* generator — deterministic, no dependencies.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn from_clock() -> Rng {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Rng { state: seed | 1 }
    }

    pub fn with_seed(seed: u64) -> Rng {
        Rng { state: seed | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform float in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform integer in `lo..=hi` (args are swapped if reversed).
    pub fn next_range(&mut self, lo: i64, hi: i64) -> i64 {
        let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
        let span = (hi as i128 - lo as i128 + 1) as u128;
        if span == 0 {
            return self.next_u64() as i64;
        }
        let pick = (self.next_u64() as u128) % span;
        (lo as i128 + pick as i128) as i64
    }
}

impl Default for Rng {
    fn default() -> Self {
        Rng::from_clock()
    }
}
