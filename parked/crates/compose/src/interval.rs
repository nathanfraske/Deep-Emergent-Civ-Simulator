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

//! The interval arithmetic the evaluator carries every port quantity in.
//!
//! Every quantity in a composed design is a bounded [`Interval`] of two [`Fixed`] (Q32.32), never a
//! single point, so an unpinned coupling (a load whose true value is not known to the bit, a
//! tolerance stack, a material property with a real spread) is represented as a width rather than a
//! fabricated exact number. The combine operations propagate the bounds: a sum widens by adding the
//! bounds, a difference widens by subtracting the crossed bounds, a product takes the extreme corners.
//! All of them are saturating in `i128`, so an out-of-range result routes to the representable
//! physical limit ([`Fixed::MAX`] or [`Fixed::MIN`]) rather than wrapping. There is no RNG and no
//! float, so the arithmetic is bit-identical on every machine.

use civsim_core::{Fixed, StateHasher};

/// Saturating add of two `Fixed` in `i128`, clamped to the representable range.
#[inline]
pub(crate) fn sat_add(a: Fixed, b: Fixed) -> Fixed {
    let s = a.to_bits() as i128 + b.to_bits() as i128;
    Fixed::from_bits(s.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
}

/// Saturating subtract of two `Fixed` in `i128`, clamped to the representable range.
#[inline]
pub(crate) fn sat_sub(a: Fixed, b: Fixed) -> Fixed {
    let s = a.to_bits() as i128 - b.to_bits() as i128;
    Fixed::from_bits(s.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
}

/// Saturating fixed-point multiply: the 128-bit product shifted back, clamped to the representable
/// range rather than wrapped, so a large product routes to the physical limit.
#[inline]
pub(crate) fn sat_mul(a: Fixed, b: Fixed) -> Fixed {
    let wide = ((a.to_bits() as i128) * (b.to_bits() as i128)) >> Fixed::FRAC_BITS;
    Fixed::from_bits(wide.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
}

/// A closed fixed-point interval `[lo, hi]`. A degenerate interval (`lo == hi`) is a pinned point.
/// The invariant `lo <= hi` is maintained by every constructor and every combine operation, so a
/// crossed pair can never represent a quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    /// The inclusive lower bound.
    pub lo: Fixed,
    /// The inclusive upper bound.
    pub hi: Fixed,
}

impl Interval {
    /// A bounded interval. The two arguments are ordered, so `new(3, 1)` is `[1, 3]`; a crossed
    /// pair is never stored.
    #[inline]
    pub fn new(lo: Fixed, hi: Fixed) -> Interval {
        if lo <= hi {
            Interval { lo, hi }
        } else {
            Interval { lo: hi, hi: lo }
        }
    }

    /// A pinned point interval `[v, v]`.
    #[inline]
    pub fn point(v: Fixed) -> Interval {
        Interval { lo: v, hi: v }
    }

    /// The zero point `[0, 0]`.
    pub const ZERO: Interval = Interval {
        lo: Fixed::ZERO,
        hi: Fixed::ZERO,
    };

    /// The width `hi - lo`, saturating. A wide interval is the evaluator's signal of an unpinnable
    /// coupling.
    #[inline]
    pub fn width(self) -> Fixed {
        sat_sub(self.hi, self.lo)
    }

    /// Whether the interval is a pinned point.
    #[inline]
    pub fn is_point(self) -> bool {
        self.lo == self.hi
    }

    /// The elementwise minimum `[min(lo1, lo2), min(hi1, hi2)]`: the weakest-link fold, where the
    /// limiting member bounds both ends. This is the interval form of "the chain is as strong as its
    /// weakest link".
    #[inline]
    pub fn min_with(self, other: Interval) -> Interval {
        Interval {
            lo: self.lo.min(other.lo),
            hi: self.hi.min(other.hi),
        }
    }

    /// Clamp both bounds into a ceiling interval, so a value can never exceed the physical envelope
    /// the caps define.
    #[inline]
    pub fn clamp_to(self, cap: Interval) -> Interval {
        Interval {
            lo: self.lo.clamp(cap.lo, cap.hi),
            hi: self.hi.clamp(cap.lo, cap.hi),
        }
    }

    /// Fold this interval's canonical bit pattern into a content hash. Two intervals with the same
    /// bounds fold identically on every machine.
    #[inline]
    pub fn hash_into(self, h: &mut StateHasher) {
        h.write_fixed(self.lo);
        h.write_fixed(self.hi);
    }
}

/// The saturating sum `[lo1 + lo2, hi1 + hi2]`: redundant, additive capacity accumulates.
impl std::ops::Add for Interval {
    type Output = Interval;
    #[inline]
    fn add(self, other: Interval) -> Interval {
        Interval::new(sat_add(self.lo, other.lo), sat_add(self.hi, other.hi))
    }
}

/// The saturating difference `[lo1 - hi2, hi1 - lo2]`: the crossed bounds, so the width of a
/// difference is the sum of the two widths (the honest worst case of an interval subtraction).
impl std::ops::Sub for Interval {
    type Output = Interval;
    #[inline]
    fn sub(self, other: Interval) -> Interval {
        Interval::new(sat_sub(self.lo, other.hi), sat_sub(self.hi, other.lo))
    }
}

/// The saturating product over the four corner products, taking the extreme low and high. This is the
/// general interval multiply, correct for a factor that may straddle zero (an efficiency interval is in
/// `[0, 1]`, but the general form is used so a signed margin can be scaled).
impl std::ops::Mul for Interval {
    type Output = Interval;
    #[inline]
    fn mul(self, other: Interval) -> Interval {
        let a = sat_mul(self.lo, other.lo);
        let b = sat_mul(self.lo, other.hi);
        let c = sat_mul(self.hi, other.lo);
        let d = sat_mul(self.hi, other.hi);
        let lo = a.min(b).min(c).min(d);
        let hi = a.max(b).max(c).max(d);
        Interval { lo, hi }
    }
}
