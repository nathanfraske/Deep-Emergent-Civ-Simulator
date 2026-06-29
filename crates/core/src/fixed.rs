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

//! Q32.32 fixed-point arithmetic, the canonical numeric type (design Part 3.1).
//!
//! All authoritative numeric state is fixed-point or integer, on the CPU and on
//! canonical GPU kernels alike. Floating point is permitted only in
//! non-authoritative work and never crosses into canonical state except through a
//! quantizer (see [`crate::canonical`]). `Fixed` is the common Q32.32 case;
//! quantities with a wider range or that accumulate over centuries carry their own
//! per-quantity scale under the unit system of design Part 55, which is built on
//! top of this type rather than replacing it.
//!
//! Part 3.1 illustrates the type as `pub type Fixed = i64` with free functions.
//! Part 58 makes the boundary a compile-time property by requiring a newtype, so
//! that a raw `f64` cannot stand in for authoritative state. This module is that
//! newtype; the free-function spelling from Part 3.1 is preserved as the methods
//! [`Fixed::mul`] and [`Fixed::div`].

use core::fmt;
use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Number of fractional bits in the Q32.32 representation.
pub const FRAC_BITS: u32 = 32;

/// A Q32.32 fixed-point number backed by `i64`.
///
/// Addition and subtraction are exact and associative, so a parallel sum of
/// `Fixed` values is independent of how the work was chunked across threads, which
/// is the property design Part 3.3 relies on to keep reductions deterministic.
/// Multiplication and division use a 128-bit intermediate to avoid overflow before
/// the shift back to Q32.32.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Fixed(i64);

impl Fixed {
    /// Fractional bits, also exposed as the module constant [`FRAC_BITS`].
    pub const FRAC_BITS: u32 = FRAC_BITS;
    /// The value zero.
    pub const ZERO: Fixed = Fixed(0);
    /// The value one (`1 << FRAC_BITS`).
    pub const ONE: Fixed = Fixed(1 << FRAC_BITS);
    /// The smallest representable value.
    pub const MIN: Fixed = Fixed(i64::MIN);
    /// The largest representable value.
    pub const MAX: Fixed = Fixed(i64::MAX);
    /// The smallest positive step (one fractional unit).
    pub const EPSILON: Fixed = Fixed(1);

    /// Construct directly from the raw `i64` bit pattern.
    #[inline]
    pub const fn from_bits(bits: i64) -> Fixed {
        Fixed(bits)
    }

    /// The raw `i64` bit pattern.
    #[inline]
    pub const fn to_bits(self) -> i64 {
        self.0
    }

    /// Construct from a whole integer. Never overflows for any `i32` input.
    #[inline]
    pub const fn from_int(i: i32) -> Fixed {
        Fixed((i as i64) << FRAC_BITS)
    }

    /// Truncate toward negative infinity to a whole integer (arithmetic shift).
    #[inline]
    pub const fn to_int(self) -> i32 {
        (self.0 >> FRAC_BITS) as i32
    }

    /// An exact fraction `num / den` in Q32.32. Panics on a zero denominator.
    #[inline]
    pub fn from_ratio(num: i64, den: i64) -> Fixed {
        Fixed((((num as i128) << FRAC_BITS) / (den as i128)) as i64)
    }

    /// Fixed-point multiply, the Part 3.1 `fx_mul`: a 128-bit intermediate then a
    /// shift back. The final narrowing cast wraps on overflow (deterministically);
    /// use [`Fixed::checked_mul`] where overflow is possible.
    #[inline]
    pub const fn mul(self, rhs: Fixed) -> Fixed {
        Fixed((((self.0 as i128) * (rhs.0 as i128)) >> FRAC_BITS) as i64)
    }

    /// Fixed-point divide, the Part 3.1 `fx_div`. Panics on a zero divisor.
    #[inline]
    pub const fn div(self, rhs: Fixed) -> Fixed {
        Fixed((((self.0 as i128) << FRAC_BITS) / (rhs.0 as i128)) as i64)
    }

    /// Checked multiply: `None` if the Q32.32 result does not fit in `i64`.
    #[inline]
    pub fn checked_mul(self, rhs: Fixed) -> Option<Fixed> {
        let wide = ((self.0 as i128) * (rhs.0 as i128)) >> FRAC_BITS;
        if wide < i64::MIN as i128 || wide > i64::MAX as i128 {
            None
        } else {
            Some(Fixed(wide as i64))
        }
    }

    /// Checked divide: `None` on a zero divisor or an out-of-range result.
    #[inline]
    pub fn checked_div(self, rhs: Fixed) -> Option<Fixed> {
        if rhs.0 == 0 {
            return None;
        }
        let wide = ((self.0 as i128) << FRAC_BITS) / (rhs.0 as i128);
        if wide < i64::MIN as i128 || wide > i64::MAX as i128 {
            None
        } else {
            Some(Fixed(wide as i64))
        }
    }

    /// Saturating add for accumulators that may run for centuries (design Part 3.1).
    #[inline]
    pub const fn saturating_add(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.saturating_add(rhs.0))
    }

    /// Wrapping add, for the rare case where modular accumulation is intended.
    #[inline]
    pub const fn wrapping_add(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.wrapping_add(rhs.0))
    }

    /// Checked add: `None` on overflow.
    #[inline]
    pub const fn checked_add(self, rhs: Fixed) -> Option<Fixed> {
        match self.0.checked_add(rhs.0) {
            Some(v) => Some(Fixed(v)),
            None => None,
        }
    }

    /// Absolute value. Panics on `Fixed::MIN` under overflow checks, as `i64` does.
    #[inline]
    pub const fn abs(self) -> Fixed {
        Fixed(self.0.abs())
    }

    /// Clamp into `[lo, hi]`.
    #[inline]
    pub fn clamp(self, lo: Fixed, hi: Fixed) -> Fixed {
        if self < lo {
            lo
        } else if self > hi {
            hi
        } else {
            self
        }
    }

    /// Non-canonical: an approximate `f64` for display, tests, and rendering only.
    /// This value must never be fed back into canonical state. The crossing back
    /// is the quantizer in [`crate::canonical`], not this method.
    #[inline]
    pub fn to_f64_lossy(self) -> f64 {
        self.0 as f64 / (1i64 << FRAC_BITS) as f64
    }

    /// Sum the raw bits of a sequence in 128-bit space. Because the accumulation is
    /// in `i128`, no intermediate grouping can overflow, so the result is identical
    /// for any order or partition of the input, even when a prefix would overflow
    /// `i64` while the total does not (audit C-05). This is the order-independent
    /// reduction the determinism contract wants for a canonical sum over a large or
    /// century-scale set; the `+` operator and the [`Sum`](core::iter::Sum) impl
    /// panic on overflow by design and are for bounded quantities only.
    #[inline]
    pub fn sum_bits<I: IntoIterator<Item = Fixed>>(iter: I) -> i128 {
        iter.into_iter().fold(0i128, |acc, x| acc + x.0 as i128)
    }

    /// Convert a 128-bit bit total back to `Fixed`, or `None` if it is out of range.
    #[inline]
    pub fn from_bits_i128(bits: i128) -> Option<Fixed> {
        if bits < i64::MIN as i128 || bits > i64::MAX as i128 {
            None
        } else {
            Some(Fixed(bits as i64))
        }
    }

    /// Order-independent checked reduction: `None` if the total is out of range. The
    /// result does not depend on how the input was partitioned across threads.
    #[inline]
    pub fn checked_sum<I: IntoIterator<Item = Fixed>>(iter: I) -> Option<Fixed> {
        Fixed::from_bits_i128(Fixed::sum_bits(iter))
    }

    /// Order-independent saturating reduction, clamping the total into range.
    #[inline]
    pub fn saturating_sum<I: IntoIterator<Item = Fixed>>(iter: I) -> Fixed {
        let bits = Fixed::sum_bits(iter).clamp(i64::MIN as i128, i64::MAX as i128);
        Fixed(bits as i64)
    }
}

impl Add for Fixed {
    type Output = Fixed;
    #[inline]
    fn add(self, rhs: Fixed) -> Fixed {
        Fixed(self.0 + rhs.0)
    }
}

impl Sub for Fixed {
    type Output = Fixed;
    #[inline]
    fn sub(self, rhs: Fixed) -> Fixed {
        Fixed(self.0 - rhs.0)
    }
}

impl Neg for Fixed {
    type Output = Fixed;
    #[inline]
    fn neg(self) -> Fixed {
        Fixed(-self.0)
    }
}

impl Mul for Fixed {
    type Output = Fixed;
    #[inline]
    fn mul(self, rhs: Fixed) -> Fixed {
        Fixed::mul(self, rhs)
    }
}

impl Div for Fixed {
    type Output = Fixed;
    #[inline]
    fn div(self, rhs: Fixed) -> Fixed {
        Fixed::div(self, rhs)
    }
}

impl AddAssign for Fixed {
    #[inline]
    fn add_assign(&mut self, rhs: Fixed) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Fixed {
    #[inline]
    fn sub_assign(&mut self, rhs: Fixed) {
        self.0 -= rhs.0;
    }
}

impl core::iter::Sum for Fixed {
    /// A fixed-order fold over the `+` operator. Because `Fixed` addition is exact
    /// and associative, the total is independent of order when every intermediate is
    /// in range. It panics on overflow by design (fail-loud), and that panic is a
    /// partial function of the grouping, so for a canonical reduction over a large
    /// or century-scale set, where a prefix could overflow while the total does not,
    /// use [`Fixed::sum_bits`] or [`Fixed::checked_sum`], which accumulate in 128-bit
    /// space and are partition-independent (audit C-05).
    fn sum<I: Iterator<Item = Fixed>>(iter: I) -> Fixed {
        iter.fold(Fixed::ZERO, |a, b| a + b)
    }
}

impl fmt::Debug for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fixed({} bits, ~{})", self.0, self.to_f64_lossy())
    }
}

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_f64_lossy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_round_trip() {
        for i in [-1000i32, -1, 0, 1, 7, 1000, i32::MAX, i32::MIN] {
            assert_eq!(Fixed::from_int(i).to_int(), i);
        }
    }

    #[test]
    fn one_is_unit() {
        assert_eq!(Fixed::from_int(1), Fixed::ONE);
        let x = Fixed::from_int(7);
        assert_eq!(x.mul(Fixed::ONE), x, "multiplying by ONE is identity");
    }

    #[test]
    fn known_products() {
        let half = Fixed::from_ratio(1, 2);
        let two = Fixed::from_int(2);
        assert_eq!(half.mul(two), Fixed::ONE, "0.5 * 2 == 1");
        assert_eq!(half.mul(half), Fixed::from_ratio(1, 4), "0.5 * 0.5 == 0.25");
        let three_halves = Fixed::from_ratio(3, 2);
        assert_eq!(three_halves.mul(two), Fixed::from_int(3), "1.5 * 2 == 3");
    }

    #[test]
    fn division_inverts_multiplication() {
        let a = Fixed::from_int(20);
        let b = Fixed::from_int(7);
        let q = a.div(b);
        // q*b reconstructs a within one fractional epsilon (fixed-point rounding).
        let recon = q.mul(b);
        let diff = (recon - a).abs();
        assert!(
            diff <= Fixed::from_bits(8),
            "reconstruction within tolerance: {diff:?}"
        );
    }

    #[test]
    fn addition_is_associative_and_commutative() {
        // The determinism contract leans on this: an order-independent reduction.
        let xs = [
            Fixed::from_ratio(1, 3),
            Fixed::from_int(5),
            Fixed::from_ratio(-7, 2),
            Fixed::from_bits(123_456_789),
            Fixed::from_int(-2),
        ];
        let left = ((xs[0] + xs[1]) + xs[2]) + (xs[3] + xs[4]);
        let right = xs[0] + (xs[1] + (xs[2] + (xs[3] + xs[4])));
        assert_eq!(left, right, "associative");
        let s1: Fixed = xs.iter().copied().sum();
        let mut rev = xs;
        rev.reverse();
        let s2: Fixed = rev.iter().copied().sum();
        assert_eq!(s1, s2, "order-independent sum");
    }

    #[test]
    fn checked_mul_detects_overflow() {
        let big = Fixed::from_bits(i64::MAX);
        assert_eq!(big.checked_mul(big), None);
        assert_eq!(
            Fixed::from_int(3).checked_mul(Fixed::from_int(4)),
            Some(Fixed::from_int(12))
        );
    }

    #[test]
    fn checked_div_handles_zero() {
        assert_eq!(Fixed::from_int(1).checked_div(Fixed::ZERO), None);
        assert_eq!(
            Fixed::from_int(6).checked_div(Fixed::from_int(2)),
            Some(Fixed::from_int(3))
        );
    }

    #[test]
    fn saturating_add_clamps() {
        assert_eq!(Fixed::MAX.saturating_add(Fixed::ONE), Fixed::MAX);
        assert_eq!(Fixed::MIN.saturating_add(Fixed::from_int(-1)), Fixed::MIN);
    }

    #[test]
    fn ordering_is_numeric() {
        assert!(Fixed::from_int(-1) < Fixed::ZERO);
        assert!(Fixed::from_ratio(1, 2) < Fixed::ONE);
        assert!(Fixed::from_int(3) > Fixed::from_ratio(5, 2));
    }
}
