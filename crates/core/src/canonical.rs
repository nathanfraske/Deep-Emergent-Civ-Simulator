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

//! The typed canonical-state boundary (design Part 58, Part 3.4).
//!
//! Determinism is enforced at compile time rather than asked of contributors to
//! remember. The [`Canonical`] marker is implemented for the fixed-point type and
//! the integer types, and deliberately not for `f32` or `f64`. A container that
//! holds authoritative state bounds its element on `Canonical`, so a float in
//! canonical state is a compile error rather than a latent nondeterminism bug.
//!
//! The only sanctioned way a non-authoritative float may enter canonical state is
//! through a quantizer that snaps it to an integer canonical unit with
//! round-half-to-even, identical across machines for the same input.

use crate::fixed::{Fixed, FRAC_BITS};

/// A type permitted in canonical (authoritative, replayable) state.
///
/// Implemented for [`Fixed`] and the integer and boolean primitives. It is
/// deliberately not implemented for `f32` or `f64`, so a generic over `Canonical`
/// cannot be instantiated with a floating-point type.
pub trait Canonical: Copy {}

impl Canonical for Fixed {}
impl Canonical for bool {}
impl Canonical for i8 {}
impl Canonical for i16 {}
impl Canonical for i32 {}
impl Canonical for i64 {}
impl Canonical for i128 {}
impl Canonical for u8 {}
impl Canonical for u16 {}
impl Canonical for u32 {}
impl Canonical for u64 {}
impl Canonical for u128 {}

/// A wrapper that marks its contents as non-authoritative. Whatever it holds can
/// never satisfy [`Canonical`], so it cannot be placed where canonical state is
/// required. Use it for render fields, language output, and view-time elaboration.
#[derive(Clone, Copy, Debug, Default)]
pub struct NonCanonical<T>(pub T);

impl<T> NonCanonical<T> {
    /// Wrap a non-authoritative value.
    pub const fn new(value: T) -> Self {
        NonCanonical(value)
    }
}

/// A cell that can only hold canonical state. The bound is the compile-time
/// boundary: `CanonicalCell::<f64>::new(..)` does not type-check, because `f64`
/// does not implement [`Canonical`].
#[derive(Clone, Copy, Debug, Default)]
pub struct CanonicalCell<T: Canonical>(T);

impl<T: Canonical> CanonicalCell<T> {
    /// Wrap a canonical value.
    pub const fn new(value: T) -> Self {
        CanonicalCell(value)
    }

    /// Read the canonical value.
    pub fn get(self) -> T {
        self.0
    }
}

/// Snap a non-authoritative `f64` into a fixed-point canonical value with
/// round-half-to-even, given how many fractional units make one whole. The result
/// is identical across machines for the same input, so the crossing is reproducible
/// (design Part 3.4).
#[inline]
pub fn quantize_unit(value: f64, units_per_one: i64) -> Fixed {
    let scaled = (value * units_per_one as f64).round_ties_even();
    // value * units_per_one is a count of fractional units; place it in the
    // Q32.32 fractional field by dividing by units_per_one in fixed-point.
    let bits = (scaled as i128 * (1i128 << FRAC_BITS)) / units_per_one as i128;
    Fixed::from_bits(bits as i64)
}

/// The Part 3.4 example: a non-authoritative water depth in metres becomes
/// canonical millimetres through a stable quantizer.
#[inline]
pub fn quantize_depth_mm(metres: f32) -> i32 {
    (metres * 1000.0).round_ties_even() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    // A generic that only accepts canonical state. Calling it with f64 would not
    // compile, which is the boundary in action.
    fn store_canonical<T: Canonical>(v: T) -> CanonicalCell<T> {
        CanonicalCell::new(v)
    }

    #[test]
    fn canonical_types_are_accepted() {
        assert_eq!(store_canonical(Fixed::ONE).get(), Fixed::ONE);
        assert_eq!(store_canonical(7i64).get(), 7);
        assert!(store_canonical(true).get());
    }

    #[test]
    fn quantize_depth_is_exact_and_stable() {
        // Values exactly representable in f32 quantize without rounding ambiguity.
        assert_eq!(quantize_depth_mm(1.5), 1500);
        assert_eq!(quantize_depth_mm(-2.5), -2500);
        assert_eq!(quantize_depth_mm(0.0), 0);
        // Determinism: same input, same output, every call.
        for _ in 0..1000 {
            assert_eq!(quantize_depth_mm(3.5), 3500);
        }
    }

    #[test]
    fn quantize_unit_lands_on_expected_fixed() {
        // 0.5 with 1000 units per one is one half in Q32.32.
        let half = quantize_unit(0.5, 1000);
        assert_eq!(half, Fixed::from_ratio(1, 2));
        let one = quantize_unit(1.0, 1000);
        assert_eq!(one, Fixed::ONE);
    }
}
