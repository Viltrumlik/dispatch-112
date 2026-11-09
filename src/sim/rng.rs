//! Small deterministic PRNG (xorshift64*), seeded through splitmix64.
//!
//! We use our own generator instead of an external crate so that the
//! simulation is bit-for-bit reproducible across platforms and crate updates.

use serde::{Deserialize, Serialize};

/// One round of splitmix64; also used to derive independent sub-seeds.
pub fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Mix two values into a new seed (order-sensitive).
pub fn mix(a: u64, b: u64) -> u64 {
    splitmix64(a ^ splitmix64(b.wrapping_add(0x5851_F42D_4C95_7F2D)))
}

/// Deterministic per-key "coin": a stable value in 0..1000 derived from a seed and keys.
pub fn permille(seed: u64, a: u64, b: u64) -> u32 {
    (mix(mix(seed, a), b) % 1000) as u32
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let s = splitmix64(seed);
        Rng { state: if s == 0 { 0x2545_F491_4F6C_DD1D } else { s } }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform integer in `0..n` (returns 0 when `n == 0`).
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        (((self.next_u64() >> 32) * n as u64) >> 32) as u32
    }

    /// Uniform integer in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + self.below((hi - lo + 1) as u32) as i32
    }

    /// True with probability `p / 1000`.
    pub fn chance(&mut self, p: u32) -> bool {
        self.below(1000) < p
    }

    /// Index chosen proportionally to `weights`; `None` if all weights are zero.
    pub fn weighted(&mut self, weights: &[u32]) -> Option<usize> {
        let total: u64 = weights.iter().map(|&w| w as u64).sum();
        if total == 0 {
            return None;
        }
        let mut roll = self.next_u64() % total;
        for (i, &w) in weights.iter().enumerate() {
            if roll < w as u64 {
                return Some(i);
            }
            roll -= w as u64;
        }
        Some(weights.len() - 1)
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            items.get(self.below(items.len() as u32) as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn below_is_in_range() {
        let mut r = Rng::new(7);
        for n in 1..50 {
            for _ in 0..50 {
                assert!(r.below(n) < n);
            }
        }
        assert_eq!(r.range(3, 3), 3);
    }

    #[test]
    fn weighted_skips_zero() {
        let mut r = Rng::new(1);
        for _ in 0..200 {
            assert_eq!(r.weighted(&[0, 5, 0]), Some(1));
        }
        assert_eq!(r.weighted(&[0, 0]), None);
    }
}
