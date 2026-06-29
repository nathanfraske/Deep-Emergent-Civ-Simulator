// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Per-entity counter-based RNG (design Part 3.2).
//!
//! There is no shared global RNG, because shared state makes results depend on the
//! order draws happen, which depends on scheduling. Each draw is a pure function of
//! a coordinate `(master_seed, entity, phase, counter)`. The same entity, in the
//! same phase, asking for its k-th number, always gets the same number on any
//! machine and at any thread count. The mixing uses a SplitMix64 finalizer so that
//! nearby entity ids do not produce correlated streams.

use crate::fixed::{Fixed, FRAC_BITS};
use crate::id::StableId;

/// The SplitMix64 finalizer used as the mixing hash (design Part 3.2).
#[inline]
pub const fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A cheap, stateless stream. The `counter` is supplied by the caller per draw, so
/// a system that needs several draws for one entity uses `at(0)`, `at(1)`, and so
/// on, and replays identically with no stored draw-count bookkeeping.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rng {
    key: u64,
}

impl Rng {
    /// Derive a stream for one entity in one phase from the master seed.
    #[inline]
    pub fn for_entity(master_seed: u64, id: StableId, phase: u32) -> Self {
        let k = splitmix64(master_seed ^ id.0.rotate_left(17));
        Rng {
            key: splitmix64(k ^ ((phase as u64) << 1)),
        }
    }

    /// Derive a stream from a raw key, for subsystems whose coordinate is not a
    /// single entity (a chunk, a region, a world-level phase).
    #[inline]
    pub const fn from_key(key: u64) -> Self {
        Rng { key }
    }

    /// The raw key of this stream.
    #[inline]
    pub const fn key(self) -> u64 {
        self.key
    }

    /// The `counter`-th 64-bit draw of this stream.
    #[inline]
    pub const fn at(self, counter: u64) -> u64 {
        splitmix64(self.key ^ counter)
    }

    /// A fixed-point fraction in `[0, ONE)` (design Part 3.2).
    #[inline]
    pub const fn unit_fixed(self, counter: u64) -> Fixed {
        // Top 32 bits scaled into the fractional field: a value in [0, ONE).
        Fixed::from_bits((self.at(counter) >> FRAC_BITS) as i64)
    }

    /// A uniform integer in `[0, n)` by Lemire's bounded method (design Part 3.2).
    /// Returns 0 when `n == 0`.
    #[inline]
    pub const fn range_u32(self, counter: u64, n: u32) -> u32 {
        (((self.at(counter) as u128) * (n as u128)) >> 64) as u32
    }

    /// A uniform integer in `[lo, hi)`. Panics if `lo >= hi`.
    #[inline]
    pub fn range_i32(self, counter: u64, lo: i32, hi: i32) -> i32 {
        assert!(lo < hi, "empty range");
        let span = (hi as i64 - lo as i64) as u32;
        lo + self.range_u32(counter, span) as i32
    }

    /// A fair coin.
    #[inline]
    pub const fn flip(self, counter: u64) -> bool {
        self.at(counter) & 1 == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::StableId;

    #[test]
    fn draws_are_pure_functions_of_their_coordinate() {
        let r = Rng::for_entity(0xDEAD_BEEF, StableId(42), 3);
        for c in 0..1000u64 {
            assert_eq!(r.at(c), r.at(c), "same coordinate, same draw");
        }
        let r2 = Rng::for_entity(0xDEAD_BEEF, StableId(42), 3);
        assert_eq!(r, r2, "stream is reconstructed identically");
        assert_eq!(r.at(7), r2.at(7));
    }

    #[test]
    fn nearby_ids_have_uncorrelated_streams() {
        let seed = 12345u64;
        let a = Rng::for_entity(seed, StableId(1000), 0);
        let b = Rng::for_entity(seed, StableId(1001), 0);
        let c = Rng::for_entity(seed, StableId(1002), 0);
        assert_ne!(a.at(0), b.at(0));
        assert_ne!(b.at(0), c.at(0));
        assert_ne!(a.key(), b.key());
    }

    #[test]
    fn phase_separates_streams() {
        let seed = 99u64;
        let p0 = Rng::for_entity(seed, StableId(7), 0);
        let p1 = Rng::for_entity(seed, StableId(7), 1);
        assert_ne!(p0.key(), p1.key());
        assert_ne!(p0.at(0), p1.at(0));
    }

    #[test]
    fn unit_fixed_in_range() {
        let r = Rng::from_key(0xABCD);
        for c in 0..10_000u64 {
            let u = r.unit_fixed(c);
            assert!(
                u >= Fixed::ZERO && u < Fixed::ONE,
                "unit fixed out of [0,1): {u:?}"
            );
        }
    }

    #[test]
    fn range_u32_bounded_and_covers() {
        let r = Rng::from_key(0x1234_5678);
        let n = 6u32;
        let mut seen = [false; 6];
        for c in 0..10_000u64 {
            let v = r.range_u32(c, n);
            assert!(v < n, "range draw out of bounds");
            seen[v as usize] = true;
        }
        assert!(seen.iter().all(|&s| s), "every value in 0..6 was produced");
        assert_eq!(r.range_u32(0, 0), 0, "n==0 yields 0");
    }

    #[test]
    fn approximate_uniformity() {
        // A light distribution sanity check, not a statistical proof.
        let r = Rng::from_key(0xFEED_FACE);
        let n = 4u32;
        let mut counts = [0u32; 4];
        let trials = 40_000u64;
        for c in 0..trials {
            counts[r.range_u32(c, n) as usize] += 1;
        }
        let expected = trials as f64 / n as f64;
        for &k in &counts {
            let dev = (k as f64 - expected).abs() / expected;
            assert!(dev < 0.1, "bucket deviation {dev} too large: {counts:?}");
        }
    }
}
