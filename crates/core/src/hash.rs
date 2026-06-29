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

//! The deterministic state hash (design Part 3.5).
//!
//! `state_hash` walks canonical state in a fixed order (sorted by [`StableId`], not
//! by hash-map iteration order, which is itself a determinism trap) and folds it
//! into a 128-bit value. The fold is FNV-1a over the 128-bit space: a pure function
//! of the byte sequence with no platform-dependent behaviour, so the same canonical
//! state hashes identically on any machine. Order matters by design, so the caller
//! is responsible for feeding state in the fixed canonical order.

use crate::fixed::Fixed;
use crate::id::StableId;

const FNV_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

/// A streaming 128-bit hash for canonical state. Feed values in a fixed order; the
/// result is a pure function of the byte sequence.
#[derive(Clone, Copy, Debug)]
pub struct StateHasher {
    state: u128,
}

impl StateHasher {
    /// A fresh hasher at the FNV-1a offset basis.
    pub fn new() -> Self {
        StateHasher { state: FNV_OFFSET }
    }

    #[inline]
    fn write_byte(&mut self, b: u8) {
        self.state = (self.state ^ b as u128).wrapping_mul(FNV_PRIME);
    }

    /// Fold in a raw byte slice.
    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_byte(b);
        }
    }

    /// Fold in a `u64` (little-endian, so the byte order is fixed across machines).
    #[inline]
    pub fn write_u64(&mut self, v: u64) {
        self.write_bytes(&v.to_le_bytes());
    }

    /// Fold in a `u32`.
    #[inline]
    pub fn write_u32(&mut self, v: u32) {
        self.write_bytes(&v.to_le_bytes());
    }

    /// Fold in an `i64`.
    #[inline]
    pub fn write_i64(&mut self, v: i64) {
        self.write_u64(v as u64);
    }

    /// Fold in a fixed-point value by its canonical bit pattern.
    #[inline]
    pub fn write_fixed(&mut self, v: Fixed) {
        self.write_i64(v.to_bits());
    }

    /// Fold in a stable id.
    #[inline]
    pub fn write_stable(&mut self, id: StableId) {
        self.write_u64(id.0);
    }

    /// The 128-bit digest so far.
    #[inline]
    pub fn finish(&self) -> u128 {
        self.state
    }
}

impl Default for StateHasher {
    fn default() -> Self {
        StateHasher::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_sequence_same_hash() {
        let mut a = StateHasher::new();
        let mut b = StateHasher::new();
        for i in 0..100u64 {
            a.write_u64(i);
            b.write_u64(i);
        }
        assert_eq!(a.finish(), b.finish());
    }

    #[test]
    fn order_changes_the_hash() {
        let mut a = StateHasher::new();
        a.write_u64(1);
        a.write_u64(2);
        let mut b = StateHasher::new();
        b.write_u64(2);
        b.write_u64(1);
        assert_ne!(
            a.finish(),
            b.finish(),
            "the hash is order-sensitive by design"
        );
    }

    #[test]
    fn empty_hash_is_the_offset_basis() {
        assert_eq!(StateHasher::new().finish(), FNV_OFFSET);
    }

    #[test]
    fn typed_writers_agree_with_raw_bytes() {
        let mut typed = StateHasher::new();
        typed.write_fixed(Fixed::from_int(3));
        typed.write_stable(StableId(7));
        let mut raw = StateHasher::new();
        raw.write_bytes(&Fixed::from_int(3).to_bits().to_le_bytes());
        raw.write_bytes(&7u64.to_le_bytes());
        assert_eq!(typed.finish(), raw.finish());
    }
}
