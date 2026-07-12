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

    /// Parse a decimal string into `Fixed` using only integer arithmetic, so the
    /// conversion is exact to the fixed-point grid and identical on every machine;
    /// floating point is never touched. This is the canonical text-to-`Fixed` reader
    /// the calibration manifest and the data-driven substrate loaders both use, so a
    /// datasheet value or a reserved number reaches canonical state losslessly.
    pub fn from_decimal_str(s: &str) -> Result<Fixed, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("empty value".to_string());
        }
        let (neg, body) = match s.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, s.strip_prefix('+').unwrap_or(s)),
        };
        let (int_str, frac_str) = match body.split_once('.') {
            Some((a, b)) => (a, b),
            None => (body, ""),
        };
        if frac_str.len() > 30 {
            return Err("too many fractional digits".to_string());
        }
        let int_val: i128 = if int_str.is_empty() {
            0
        } else {
            int_str
                .parse::<i128>()
                .map_err(|e| format!("bad integer part: {e}"))?
        };
        let mut bits: i128 = int_val << FRAC_BITS;
        if !frac_str.is_empty() {
            let digits: i128 = frac_str
                .parse::<i128>()
                .map_err(|e| format!("bad fractional part: {e}"))?;
            let mut den: i128 = 1;
            for _ in 0..frac_str.len() {
                den *= 10;
            }
            bits += (digits << FRAC_BITS) / den;
        }
        if neg {
            bits = -bits;
        }
        if bits < i64::MIN as i128 || bits > i64::MAX as i128 {
            return Err("value out of Q32.32 range".to_string());
        }
        Ok(Fixed(bits as i64))
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

    /// The non-negative square root, exact to the last fixed-point bit and deterministic.
    /// Computed by an integer square root over the 128-bit radicand `bits << FRAC_BITS`
    /// (since `sqrt(b / 2^F) * 2^F = isqrt(b * 2^F)`), so there is no float and the result
    /// is identical on every machine and thread count. A negative input has no real root and
    /// returns [`Fixed::ZERO`]; callers that can pass a negative (rather than a sum of
    /// squares, which cannot) should guard first.
    #[inline]
    pub fn sqrt(self) -> Fixed {
        if self.0 <= 0 {
            return Fixed::ZERO;
        }
        let radicand = (self.0 as u128) << FRAC_BITS;
        Fixed(radicand.isqrt() as i64)
    }

    /// The real cube root, exact to the last fixed-point bit and deterministic, and defined for a
    /// negative input (the cube root of a negative is real, unlike the square root). Computed by an
    /// integer cube root over the 128-bit radicand `|bits| << 2*FRAC_BITS` (since
    /// `cbrt(b / 2^F) * 2^F = icbrt(b * 2^(2F))`), sign carried through, so there is no float and no
    /// transcendental series, and the result is identical on every machine and thread count. This is
    /// the exact sibling to [`Fixed::sqrt`]: where `self.powf(1/3)` computes a cube root as the
    /// exponential of a third of the logarithm and carries the compounded rounding of two series,
    /// this floors a single integer root with no series error, so a perfect cube returns exactly.
    #[inline]
    pub fn cbrt(self) -> Fixed {
        if self.0 == 0 {
            return Fixed::ZERO;
        }
        // The magnitude's radicand: |bits| shifted by 2*FRAC_BITS. |bits| is at most 2^63, so the
        // shift by 64 stays under 2^127 and fits the u128 radicand.
        let radicand = (self.0.unsigned_abs() as u128) << (2 * FRAC_BITS);
        let root = integer_cbrt(radicand) as i64;
        Fixed(if self.0 < 0 { -root } else { root })
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

/// The integer cube root of a `u128`: the largest `r` with `r^3 <= n`, computed by a bounded binary
/// search. Every step is exact integer arithmetic (the cube guarded by `checked_mul`, so a candidate
/// whose cube overflows `u128` is correctly treated as too large), so the result is deterministic and
/// identical on every machine. The upper bound `1 << 43` covers the whole `u128` range, since
/// `(2^43)^3 = 2^129 > u128::MAX`. This is the cube-root counterpart of the `u128::isqrt` the square
/// root uses, hand-written because the standard library provides no integer cube root.
fn integer_cbrt(n: u128) -> u128 {
    let (mut lo, mut hi) = (0u128, 1u128 << 43);
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        let cube = mid.checked_mul(mid).and_then(|sq| sq.checked_mul(mid));
        match cube {
            Some(c) if c <= n => lo = mid,
            _ => hi = mid - 1,
        }
    }
    lo
}

// === Transcendental constants (Q32.32, round-to-nearest of c*2^32, embedded exactly and
// deterministically as raw bits; the pinned reference for R-GPU-CANON-PIN). ===
const LN2: Fixed = Fixed::from_bits(2977044472); // ln 2
const INV_LN2: Fixed = Fixed::from_bits(6196328019); // 1/ln 2
const PI_BITS: Fixed = Fixed::from_bits(13493037705); // pi
const HALF_PI_BITS: Fixed = Fixed::from_bits(6746518852); // pi/2
const HALF: Fixed = Fixed::from_bits(1 << 31); // 0.5, for round-to-nearest
const CORDIC_INV_GAIN: Fixed = Fixed::from_bits(2608131496); // 1/A_32, the CORDIC prescale
const CORDIC_N: usize = 32;
const CORDIC_ATAN: [Fixed; CORDIC_N] = [
    Fixed::from_bits(3373259426),
    Fixed::from_bits(1991351318),
    Fixed::from_bits(1052175346),
    Fixed::from_bits(534100635),
    Fixed::from_bits(268086748),
    Fixed::from_bits(134174063),
    Fixed::from_bits(67103403),
    Fixed::from_bits(33553749),
    Fixed::from_bits(16777131),
    Fixed::from_bits(8388597),
    Fixed::from_bits(4194303),
    Fixed::from_bits(2097152),
    Fixed::from_bits(1048576),
    Fixed::from_bits(524288),
    Fixed::from_bits(262144),
    Fixed::from_bits(131072),
    Fixed::from_bits(65536),
    Fixed::from_bits(32768),
    Fixed::from_bits(16384),
    Fixed::from_bits(8192),
    Fixed::from_bits(4096),
    Fixed::from_bits(2048),
    Fixed::from_bits(1024),
    Fixed::from_bits(512),
    Fixed::from_bits(256),
    Fixed::from_bits(128),
    Fixed::from_bits(64),
    Fixed::from_bits(32),
    Fixed::from_bits(16),
    Fixed::from_bits(8),
    Fixed::from_bits(4),
    Fixed::from_bits(2),
];

/// Multiply a `Fixed` by 2^k (k signed), saturating on overflow, flooring on a negative shift.
#[inline]
fn scale_pow2(v: Fixed, k: i32) -> Fixed {
    if k >= 0 {
        let s = (v.0 as i128) << k;
        if s > i64::MAX as i128 {
            Fixed::MAX
        } else if s < i64::MIN as i128 {
            Fixed::MIN
        } else {
            Fixed(s as i64)
        }
    } else {
        Fixed(v.0 >> (-k) as u32)
    }
}

/// CORDIC circular rotation: given an angle in about `[-pi/4, pi/4]`, return `(cos, sin)`. Shift-add
/// only, `CORDIC_N` fixed iterations, so the result is bit-identical on every machine and backend.
#[inline]
fn cordic_rotation(theta: Fixed) -> (Fixed, Fixed) {
    let mut x = CORDIC_INV_GAIN;
    let mut y = Fixed::ZERO;
    let mut z = theta;
    let mut i = 0usize;
    while i < CORDIC_N {
        let dx = Fixed(x.0 >> i);
        let dy = Fixed(y.0 >> i);
        if z.0 >= 0 {
            x -= dy;
            y += dx;
            z -= CORDIC_ATAN[i];
        } else {
            x += dy;
            y -= dx;
            z += CORDIC_ATAN[i];
        }
        i += 1;
    }
    (x, y)
}

/// CORDIC vectoring: return `atan(y0/x0)` for `x0 > 0`, driving `y` to zero and accumulating the
/// angle. Shift-add only, `CORDIC_N` fixed iterations.
#[inline]
fn cordic_vectoring(mut x: Fixed, mut y: Fixed) -> Fixed {
    let mut z = Fixed::ZERO;
    let mut i = 0usize;
    while i < CORDIC_N {
        let dx = Fixed(x.0 >> i);
        let dy = Fixed(y.0 >> i);
        if y.0 >= 0 {
            x += dy;
            y -= dx;
            z += CORDIC_ATAN[i];
        } else {
            x -= dy;
            y += dx;
            z -= CORDIC_ATAN[i];
        }
        i += 1;
    }
    z
}

impl Fixed {
    /// The circle constant pi, as a Q32.32 constant.
    pub const PI: Fixed = PI_BITS;
    /// Half pi (a right angle), as a Q32.32 constant.
    pub const HALF_PI: Fixed = HALF_PI_BITS;

    /// Round to the nearest integer (half up), for range reduction.
    #[inline]
    fn round_to_int(self) -> i32 {
        (self + HALF).to_int()
    }

    /// The natural exponential `e^x`, integer-only and deterministic (the pinned R-GPU-CANON-PIN
    /// reference). Range-reduces `x = k*ln2 + r`, evaluates a Maclaurin series on `r` by integer
    /// Horner, and scales by `2^k` with an exact shift. Outside the representable window (about
    /// `[-22, 21.5]`) it saturates to zero or the maximum, an honest Q32.32 limit.
    pub fn exp(self) -> Fixed {
        if self > Fixed::from_int(22) {
            return Fixed::MAX;
        }
        if self < Fixed::from_int(-22) {
            return Fixed::ZERO;
        }
        let k = self.mul(INV_LN2).to_int();
        let r = self - Fixed::from_int(k).mul(LN2);
        // exp(r) = 1 + r(1 + r/2 (1 + r/3 (...))), Horner over the Maclaurin series.
        let mut acc = Fixed::ONE;
        let mut i = 18i32;
        while i >= 1 {
            acc = Fixed::ONE + r.mul(acc).div(Fixed::from_int(i));
            i -= 1;
        }
        scale_pow2(acc, k)
    }

    /// The natural logarithm `ln(x)`, integer-only and deterministic. Normalizes `x = m*2^e` with
    /// `m` in `[1,2)` by a leading-bit scan, then `ln(x) = e*ln2 + ln(m)` with `ln(m)` from the fast
    /// `atanh` series on `(m-1)/(m+1)`. A non-positive input has no real log and returns
    /// [`Fixed::MIN`] as a fail-loud sentinel; callers guard their domain.
    pub fn ln(self) -> Fixed {
        if self.0 <= 0 {
            return Fixed::MIN;
        }
        let b = self.0;
        let msb = 63 - b.leading_zeros() as i32;
        let e = msb - FRAC_BITS as i32;
        let m = if e >= 0 {
            Fixed(b >> e as u32)
        } else {
            Fixed(b << (-e) as u32)
        };
        let u = (m - Fixed::ONE).div(m + Fixed::ONE);
        let w = u.mul(u);
        // sum_{j=0}^{J} w^j / (2j+1), Horner.
        let mut acc = Fixed::ZERO;
        let mut j = 12i32;
        while j >= 0 {
            acc = acc.mul(w) + Fixed::ONE.div(Fixed::from_int(2 * j + 1));
            j -= 1;
        }
        let ln_m = Fixed::from_int(2).mul(u).mul(acc);
        Fixed::from_int(e).mul(LN2) + ln_m
    }

    /// Sine and cosine together, integer-only and deterministic (CORDIC). Reduces the angle to
    /// `[-pi/4, pi/4]` by quadrant, rotates, and maps back. Returns `(sin, cos)`.
    #[inline]
    pub fn sin_cos(self) -> (Fixed, Fixed) {
        let n = self.div(HALF_PI_BITS).round_to_int();
        let r = self - Fixed::from_int(n).mul(HALF_PI_BITS);
        let (c, s) = cordic_rotation(r);
        match n.rem_euclid(4) {
            0 => (s, c),
            1 => (c, Fixed::ZERO - s),
            2 => (Fixed::ZERO - s, Fixed::ZERO - c),
            _ => (Fixed::ZERO - c, s),
        }
    }

    /// The sine of an angle in radians.
    #[inline]
    pub fn sin(self) -> Fixed {
        self.sin_cos().0
    }

    /// The cosine of an angle in radians.
    #[inline]
    pub fn cos(self) -> Fixed {
        self.sin_cos().1
    }

    /// The arctangent `atan(x)` in radians, in `(-pi/2, pi/2)`, integer-only and deterministic
    /// (CORDIC vectoring). A very large magnitude saturates toward the right angle.
    pub fn atan(self) -> Fixed {
        let bound = Fixed::from_int(1 << 28);
        if self > bound {
            return Fixed::HALF_PI;
        }
        if self < Fixed::ZERO - bound {
            return Fixed::ZERO - Fixed::HALF_PI;
        }
        cordic_vectoring(Fixed::ONE, self)
    }

    /// The arcsine `asin(x)` in radians, in `[-pi/2, pi/2]`. Outside the domain `[-1, 1]` (the
    /// physical total-internal-reflection boundary) it saturates to the right angle. Computed as
    /// `atan(x / sqrt(1 - x*x))`.
    pub fn asin(self) -> Fixed {
        if self >= Fixed::ONE {
            return Fixed::HALF_PI;
        }
        if self <= Fixed::from_int(-1) {
            return Fixed::ZERO - Fixed::HALF_PI;
        }
        let denom = (Fixed::ONE - self.mul(self)).sqrt();
        cordic_vectoring(denom, self)
    }

    /// Integer power `x^n` by exponentiation-by-squaring, exact to the multiply rounding and
    /// deterministic. A negative exponent takes the reciprocal first.
    pub fn powi(self, n: i32) -> Fixed {
        if n == 0 {
            return Fixed::ONE;
        }
        let mut base = if n < 0 { Fixed::ONE.div(self) } else { self };
        let mut e = n.unsigned_abs();
        let mut acc = Fixed::ONE;
        while e > 0 {
            if e & 1 == 1 {
                acc = acc.mul(base);
            }
            e >>= 1;
            if e > 0 {
                base = base.mul(base);
            }
        }
        acc
    }

    /// Real power `x^y = exp(y * ln x)` for `x > 0`, composed from the pinned `ln` and `exp`. A
    /// non-positive base returns zero (the domain guard).
    pub fn powf(self, y: Fixed) -> Fixed {
        if self <= Fixed::ZERO {
            return Fixed::ZERO;
        }
        y.mul(self.ln()).exp()
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
    fn sqrt_is_exact_on_squares_and_deterministic() {
        // Perfect squares come back exactly.
        assert_eq!(Fixed::from_int(4).sqrt(), Fixed::from_int(2));
        assert_eq!(Fixed::from_int(9).sqrt(), Fixed::from_int(3));
        assert_eq!(Fixed::ONE.sqrt(), Fixed::ONE);
        assert_eq!(Fixed::ZERO.sqrt(), Fixed::ZERO);
        // sqrt(2) squares back to within fixed-point tolerance, and is the floored root.
        let r = Fixed::from_int(2).sqrt();
        let sq = r.mul(r);
        assert!((sq - Fixed::from_int(2)).abs() <= Fixed::from_bits(1 << 17));
        assert!(
            sq <= Fixed::from_int(2),
            "isqrt floors, so the square does not exceed 2"
        );
        // A negative input has no real root and returns zero (guarded, not a panic).
        assert_eq!(Fixed::from_int(-5).sqrt(), Fixed::ZERO);
        // Deterministic: the same input gives the same bits every call.
        assert_eq!(
            Fixed::from_ratio(7, 3).sqrt(),
            Fixed::from_ratio(7, 3).sqrt()
        );
    }

    #[test]
    fn cbrt_is_exact_on_cubes_floors_to_the_last_bit_and_matches_powf() {
        // Perfect cubes come back exactly (no series error, unlike powf(1/3)).
        assert_eq!(Fixed::from_int(8).cbrt(), Fixed::from_int(2));
        assert_eq!(Fixed::from_int(27).cbrt(), Fixed::from_int(3));
        assert_eq!(Fixed::from_int(64).cbrt(), Fixed::from_int(4));
        assert_eq!(Fixed::from_int(1000).cbrt(), Fixed::from_int(10));
        assert_eq!(Fixed::ONE.cbrt(), Fixed::ONE);
        assert_eq!(Fixed::ZERO.cbrt(), Fixed::ZERO);
        // Defined for negatives (the cube root of a negative is real): cbrt(-x) = -cbrt(x).
        assert_eq!(Fixed::from_int(-8).cbrt(), Fixed::from_int(-2));
        assert_eq!(Fixed::from_int(-27).cbrt(), Fixed::from_int(-3));

        // The root floors to the last fixed-point bit: its cube does not exceed the input radicand,
        // and the next representable bit up would, proven in exact u128 integer arithmetic (no float).
        for x in [
            Fixed::from_int(2),
            Fixed::from_int(50),
            Fixed::from_ratio(7, 3),
            Fixed::from_int(1_000_000),
        ] {
            let rb = x.cbrt().to_bits() as u128;
            let radicand = (x.to_bits() as u128) << (2 * FRAC_BITS);
            assert!(
                rb.pow(3) <= radicand,
                "the cube of the root does not exceed the input"
            );
            assert!(
                (rb + 1).pow(3) > radicand,
                "the next bit up would exceed it (floored to the last bit)"
            );
        }

        // Matches the powf(1/3) approximation to the powf precision target (cbrt is the exact one).
        let third = Fixed::from_ratio(1, 3);
        for x in [
            Fixed::from_int(8),
            Fixed::from_int(100),
            Fixed::from_int(1000),
        ] {
            let exact = x.cbrt();
            let approx = x.powf(third);
            assert!(
                (exact - approx).abs() <= Fixed::from_ratio(1, 100),
                "cbrt {exact:?} matches powf(1/3) {approx:?} to the powf target"
            );
        }

        // Deterministic: the same input gives the same bits every call.
        assert_eq!(
            Fixed::from_ratio(7, 3).cbrt(),
            Fixed::from_ratio(7, 3).cbrt()
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
