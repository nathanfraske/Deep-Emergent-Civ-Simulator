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

/// THE OUTCOME OF AN EXPONENTIAL IN FIXED POINT, distinguishing the three ways it can fail to be a number.
///
/// # WHY A TYPE RATHER THAN A SENTINEL
///
/// [`Fixed::exp`] saturates to [`Fixed::MAX`] and flushes to [`Fixed::ZERO`] at its rails, and the first
/// attempt at a checked exponential SNIFFED those values back afterwards. That cannot work through a
/// wrapping operation: if the argument itself overflowed on the way in, `exp` receives a plausible WRAPPED
/// number, returns a plausible finite result, and there is no sentinel left to read. The loss is already
/// unrecoverable by the time anything looks.
///
/// So the classification happens on the ARGUMENT, before evaluation, and it is carried in the type.
///
/// # THE TWO FAILURES ARE NOT THE SAME AND CALLERS MUST NOT TREAT THEM ALIKE
///
/// [`Self::Overflow`] is unbounded error: the true value is past what the representation holds and nothing
/// about the returned number relates to it. [`Self::Underflow`] is bounded by about one ulp: the true value
/// was already below the grid, so ZERO is very nearly right and is often the answer a caller wants. Collapsing
/// both into one refusal makes a conservative floor look like a caught defect and hides the real one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpOutcome {
    /// The exponential is representable and this is its value.
    Finite(Fixed),
    /// The true value lies below one ulp. Zero is within an ulp of correct; a caller that wants the floor
    /// may take it, and one that wants to know it happened now can.
    Underflow,
    /// The true value is past [`Fixed::MAX`]. Nothing representable relates to it.
    Overflow,
    /// The input is outside the function's domain (for a power, a non-positive base).
    Domain,
}

impl ExpOutcome {
    /// The value where the exponential is finite, and `None` at either rail or outside the domain. The
    /// lossy view, for a caller with no distinct policy.
    pub fn finite(self) -> Option<Fixed> {
        match self {
            ExpOutcome::Finite(v) => Some(v),
            _ => None,
        }
    }
}

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

    /// Checked subtract: `None` if the difference does not fit in `i64` (a raw fixed-point subtraction, no
    /// scaling). Completes the checked-arithmetic family alongside [`Fixed::checked_mul`]/[`Fixed::checked_div`],
    /// so a `pub` kernel can stay total on a difference that would panic the `Sub` operator under overflow-checks.
    #[inline]
    pub fn checked_sub(self, rhs: Fixed) -> Option<Fixed> {
        self.0.checked_sub(rhs.0).map(Fixed)
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
    ///
    /// # IT SATURATES SILENTLY, and that is inherited rather than chosen
    ///
    /// [`Fixed::exp`] saturates to [`Fixed::MAX`] above an argument of about 21.5, which its own
    /// documentation states as an honest Q32.32 limit. This function is `exp(y ln x)`, so it inherits
    /// that rail whenever `y ln x` crosses it, and until 2026-07-19 it did not say so. A caller reading
    /// only this comment had no way to learn it.
    ///
    /// MEASURED, because the threshold is what a caller needs: at the Dohnanyi slope `y = 2.5`, a base
    /// spanning 3.8 decades already rails, and at 6 decades this returns `2.147e9` where the true value
    /// is `1e15`. Six orders of magnitude, with no error and no signal.
    ///
    /// WHAT IS AND IS NOT WRONG HERE, stated precisely because the loose version overstates it. The rail
    /// condition is `y ln x > 21.5`, which is `x^y > e^21.5`, which is about `Fixed::MAX`. So this
    /// saturates EXACTLY when its result stops being representable: the arithmetic is not wrong, the
    /// SIGNAL is missing. It cannot distinguish "your answer is 2.147e9" from "your answer does not fit",
    /// and it reports the former in both cases.
    ///
    /// That is not hypothetical. Two functions in `world::impact_flux` documented a fail-soft on a
    /// too-wide size range, did not have one because of this rail, and returned `1.5e-2` where the truth
    /// was `3.2e-8`, five and a half orders high, in a value a caller would have believed.
    ///
    /// SURVEYED ACROSS THE TREE, so the exposure is a measurement rather than an alarm. Budgets below are
    /// MAGNITUDES of `y ln x`, against a window about 22 wide either way. Of the shapes the engine calls:
    /// the crater strength group reaches `13.9` (toward the underflow side), the Mie-Grueneisen volume
    /// ratio `8.6` at the steepest banked `q` (also underflow), the disk surface density `5.6`, the
    /// metallicity dex power `2.8` (toward saturation), and the Birch-Murnaghan cube root `0.4`. The full
    /// site-by-site measurement, with the input bound behind each number and which rail each approaches,
    /// is `docs/working/POWF_CALL_SITE_AUDIT.md`.
    ///
    /// TWO SHAPES REACH A RAIL. The impact size-frequency ratio spends `34.5` over six decades of size,
    /// and `world::impact_flux` now guards it (a too-wide reservoir refuses rather than answering). The
    /// stellar mass-luminosity power `M^3.5` rails above `464` solar masses, and it was found ESCAPING:
    /// the railed luminosity reaches a caller as a plausible flux at distance, `166x` low at `2000` solar
    /// masses with no signal. Both call sites now read the sentinel back.
    ///
    /// THERE ARE TWO RAILS, and they are not the same kind of failure. The upper one saturates to
    /// [`Fixed::MAX`] when `y ln x > 21.4875626` (measured, the point where `exp` exceeds `2^31`), and its
    /// error is UNBOUNDED: at `(1e8)^2.5` this returns `2.147e9` against a truth of `1e20`. The lower one
    /// underflows to [`Fixed::ZERO`] when `y ln x < -22`, and its error is bounded by roughly ONE ULP,
    /// because `e^-22 = 2.79e-10` and one ulp is `2.33e-10`, so a zeroed result was never more than 1.2
    /// grid steps from zero to begin with. Treat the upper rail as a defect to guard and the lower one as
    /// the representable floor doing its job.
    ///
    /// THE EXPONENT WINDOW, the cheapest bound a caller can check. `ln` of a representable positive
    /// `Fixed` lies in `[-22.1807098, 21.4875626]` (the ulp and [`Fixed::MAX`]), so an exponent inside
    /// `[-0.9687499, +0.9918528]` CANNOT reach either rail for ANY representable base, no matter what the
    /// base is. Every cube root, fourth root, and fractional exponent in the tree is covered by that one
    /// fact and needs no further argument. Outside the window the caller owes a bound on the BASE.
    ///
    /// Use [`Fixed::checked_powf`] where the result feeds a decision. Reach for this one only where the
    /// operands are bounded by something that keeps `y ln x` inside the window, and say what that is.
    pub fn powf(self, y: Fixed) -> Fixed {
        if self <= Fixed::ZERO {
            return Fixed::ZERO;
        }
        y.mul(self.ln()).exp()
    }

    /// `x^y`, or `None` where the composition rails.
    ///
    /// Reads [`Fixed::exp`]'s saturation sentinel back rather than trusting the value: a result of
    /// exactly [`Fixed::MAX`] from a finite argument means the exponential clamped, so the true power is
    /// somewhere above the representable window and no number here is honest. A non-positive base
    /// returns `None` rather than [`Fixed::powf`]'s zero, because a domain violation is not a result.
    ///
    /// THE TWO FALSE POSITIVES, stated so they are not discovered later. A computation whose true value
    /// lands exactly on `Fixed::MAX` is indistinguishable from a rail and is refused. A computation whose
    /// true value falls below one ulp is indistinguishable from an underflow and is refused, where the
    /// bare form would return the zero that is in fact the correct representable answer. Both are the
    /// right trade: a wrong number that looks right costs more than a refusal on a boundary a caller can
    /// widen.
    pub fn checked_powf(self, y: Fixed) -> Option<Fixed> {
        self.powf_outcome(y).finite()
    }

    /// `self^y` WITH ITS FAILURE CLASSIFIED, which is what [`Self::checked_powf`] discards.
    ///
    /// # THE PRODUCT IS CHECKED, AND THAT IS THE HALF THE FIRST VERSION MISSED
    ///
    /// `powf` is `exp(y ln x)`, and the first checked form read `y.mul(self.ln()).exp()` and then compared
    /// the RESULT against the rails. `mul` is the WRAPPING multiply. When `y ln x` itself left the window it
    /// wrapped to a plausible argument, `exp` returned a plausible finite number, and no sentinel survived
    /// for the check to find: the answer was already wrong and no longer detectably so. The product is now
    /// `checked_mul`, so an overflow there is caught where it happens instead of being laundered through an
    /// exponential.
    ///
    /// # THE RAILS ARE READ ON THE ARGUMENT, NOT SNIFFED FROM THE VALUE
    ///
    /// [`Fixed::exp`] saturates rather than wrapping, so the classification is a comparison on `y ln x`
    /// before evaluating rather than an equality test on something that may legitimately BE a rail. A true
    /// result landing exactly on `Fixed::MAX` used to be refused as a false positive; it is `Finite` now,
    /// because the argument test can tell them apart.
    ///
    /// # THE EDGE IS DERIVED, AND THE AUTHORED 22 WAS THE SECOND HALF OF THE SAME DEFECT
    ///
    /// The first version of this test read `arg > 22`, copied from [`Fixed::exp`]'s own saturation guard. 22
    /// is not the representable edge: `exp` reaches `Fixed::MAX` at [`Self::exp_max_argument`],
    /// `ln(Fixed::MAX) = 21.4875626`. Across the whole band `(21.4875626, 22]` the argument test called the
    /// result `Finite` while [`Fixed::exp`]'s `scale_pow2` had already saturated it to `Fixed::MAX`, so a
    /// railed value was returned as an answer. That is the laundering this function exists to stop,
    /// reintroduced one layer up by an authored constant standing where a derived one belongs. The mass
    /// luminosity `M^3.5` lands in that band at 464 solar masses (`3.5 ln 464 = 21.4896`), which is how it
    /// surfaced.
    // @derives: a classified fixed-point power <- the base, the exponent and the representable window
    pub fn powf_outcome(self, y: Fixed) -> ExpOutcome {
        if self <= Fixed::ZERO {
            return ExpOutcome::Domain;
        }
        let Some(arg) = y.checked_mul(self.ln()) else {
            // The exponent times the log left the window. Its SIGN tells which rail it was heading for, and
            // the operands are what decide it: a positive product overflows, a negative one underflows.
            let positive = (y > Fixed::ZERO) == (self > Fixed::ONE);
            return if positive {
                ExpOutcome::Overflow
            } else {
                ExpOutcome::Underflow
            };
        };
        arg.exp_outcome()
    }

    /// The largest argument whose exponential is representable, DERIVED from the representation rather than
    /// authored: `exp(x)` exceeds [`Fixed::MAX`] exactly when `x` exceeds `ln(Fixed::MAX)`.
    ///
    /// Measured against what [`Fixed::exp`] actually does rather than trusted from the algebra: the last
    /// argument whose exponential lands below the rail is `21.487560871` and the first that reaches it is
    /// `21.487563870`, so this derived edge falls inside the measured crossover. Within the `1.5e-6` sliver
    /// below it the series rails about `2e-6` early in relative terms, which is the accuracy the top of the
    /// range carries anyway.
    // @derives: the representable exponential edge <- the natural log of the representation's ceiling
    pub fn exp_max_argument() -> Fixed {
        Fixed::MAX.ln()
    }

    /// [`Fixed::exp`] WITH ITS FAILURE CLASSIFIED, the sibling of [`Self::powf_outcome`] for a caller that
    /// already holds the exponent.
    ///
    /// The underflow edge stays at `-22`, which is [`Fixed::exp`]'s own floor rather than the
    /// representation's: the smallest positive `Fixed` is `2^-32`, whose log is `-22.1807`. Between the two
    /// the true value is under one ulp, so zero is very nearly right and `Underflow` is the honest report.
    /// The overflow edge is the derived one, because there the gap is the whole distance to the rail.
    // @derives: a classified fixed-point exponential <- the argument and the representable window
    pub fn exp_outcome(self) -> ExpOutcome {
        if self > Fixed::exp_max_argument() {
            return ExpOutcome::Overflow;
        }
        if self < Fixed::from_int(-22) {
            return ExpOutcome::Underflow;
        }
        ExpOutcome::Finite(self.exp())
    }

    /// The COMPLEMENTARY ERROR FUNCTION `erfc(x) = 1 - erf(x)`, by Abramowitz and Stegun 7.1.26 (a
    /// fixed five-term rational in `t = 1/(1 + p x)` times `exp(-x^2)`, maximum error 1.5e-7),
    /// fixed-form and deterministic. Negative arguments reflect through `erfc(-x) = 2 - erfc(x)`,
    /// so the whole real line is covered by the one non-negative branch the fit is stated for.
    ///
    /// The coefficients are the A&S 7.1.26 fit itself, the form's own numbers rather than authored
    /// values: this is a named approximation carrying its citation, the same standing as the Petit
    /// surface's ensemble constants or the Kelvin series' Euler-Mascheroni gamma.
    ///
    /// It lives here, beside `exp`/`ln`/`sqrt`/`cbrt`/`powf`, because it is a transcendental of the
    /// numeric type rather than a fact about any one domain. Two consumers need it and must not
    /// drift apart: Ewald's real-space sum (where the argument `alpha r` is always non-negative) and
    /// the half-space cooling geotherm (`erf(z / (2 sqrt(kappa t)))`, the lid profile).
    pub fn erfc(self) -> Fixed {
        if self < Fixed::ZERO {
            return Fixed::from_int(2) - (Fixed::ZERO - self).erfc();
        }
        let p = Fixed::from_ratio(3_275_911, 10_000_000);
        let a1 = Fixed::from_ratio(254_829_592, 1_000_000_000);
        let a2 = Fixed::from_ratio(-284_496_736, 1_000_000_000);
        let a3 = Fixed::from_ratio(1_421_413_741, 1_000_000_000);
        let a4 = Fixed::from_ratio(-1_453_152_027, 1_000_000_000);
        let a5 = Fixed::from_ratio(1_061_405_429, 1_000_000_000);
        let t = Fixed::ONE / (Fixed::ONE + p * self);
        // Horner: t * (a1 + t*(a2 + t*(a3 + t*(a4 + t*a5)))).
        let poly = t * (a1 + t * (a2 + t * (a3 + t * (a4 + t * a5))));
        poly * (Fixed::ZERO - self * self).exp()
    }

    /// The ERROR FUNCTION `erf(x) = 1 - erfc(x)`, composed from the pinned [`Fixed::erfc`] so the
    /// two are one implementation and cannot disagree. `erf(0) = 0`, `erf(x) -> 1` as `x` grows, and
    /// it is odd (`erf(-x) = -erf(x)`, inherited from `erfc`'s reflection).
    pub fn erf(self) -> Fixed {
        Fixed::ONE - self.erfc()
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
mod powf_rail_tests {
    use super::*;

    /// THE RAIL IS REAL AND `checked_powf` CATCHES IT, measured at the threshold rather than asserted.
    ///
    /// `powf` is `exp(y ln x)` and `exp` saturates above about 21.5, so a wide enough base rails. This
    /// pins the measured behaviour so a future change to `exp`'s window cannot move it silently.
    #[test]
    fn powf_rails_where_checked_powf_refuses() {
        let exponent = Fixed::from_ratio(25, 10); // the Dohnanyi slope's p - 1
                                                  // Inside the window: both agree.
        for decades in [1i32, 2, 3] {
            let base = Fixed::from_int(10i32.pow(decades as u32));
            let bare = base.powf(exponent);
            let checked = base
                .checked_powf(exponent)
                .expect("inside the window it answers");
            assert_eq!(bare, checked, "{decades} decades must agree");
            assert_ne!(bare, Fixed::MAX, "{decades} decades must not rail");
        }
        // Past it: the bare form returns MAX and lies; the checked form refuses.
        for decades in [4i32, 5, 6] {
            let base = Fixed::from_int(10i32.pow(decades as u32));
            assert_eq!(
                base.powf(exponent),
                Fixed::MAX,
                "{decades} decades rails the bare form, which is the defect being documented"
            );
            assert!(
                base.checked_powf(exponent).is_none(),
                "{decades} decades must REFUSE rather than return the rail"
            );
        }
    }

    /// THE EXPONENT WINDOW, the bound that classifies most call sites without any argument about the base.
    ///
    /// `ln` of a representable positive `Fixed` lies in `[ln(ulp), ln(MAX)] = [-22.1807098, 21.4875626]`,
    /// so `y ln x` is bounded by the exponent alone. An exponent inside `[-0.9687499, +0.9918528]` cannot
    /// reach either rail for ANY representable base. This SWEEPS the whole representable range (every
    /// power-of-two magnitude, both extremes) at each edge and one step past it, so the window is measured
    /// rather than derived, and a future change to `exp`'s or `ln`'s window moves this test.
    ///
    /// The two edges are different because the rails are not symmetric: the upper rail sits at
    /// `+21.4875626` and the lower at `-22`, and a negative exponent maps the MOST negative log (the ulp,
    /// `-22.1807098`) onto the UPPER rail, which is why the negative edge is the tighter of the two.
    #[test]
    fn the_exponent_window_cannot_rail_for_any_representable_base() {
        // Spanning set: every power-of-two magnitude in the representable range, plus both extremes.
        let mut bases: Vec<Fixed> = (0..63).map(|k| Fixed::from_bits(1i64 << k)).collect();
        bases.push(Fixed::MAX);
        bases.push(Fixed::from_bits(1));
        bases.push(Fixed::ONE);
        let rails = |y: Fixed| -> bool {
            bases
                .iter()
                .filter(|b| **b > Fixed::ZERO)
                .any(|b| b.powf(y) == Fixed::MAX || b.powf(y) == Fixed::ZERO)
        };

        // Inside the window, at both edges: nothing rails.
        let positive_edge = Fixed::from_ratio(9_918_528, 10_000_000);
        let negative_edge = Fixed::ZERO - Fixed::from_ratio(9_687_499, 10_000_000);
        assert!(
            !rails(positive_edge),
            "the positive edge +0.9918528 must clear both rails for every representable base"
        );
        assert!(
            !rails(negative_edge),
            "the negative edge -0.9687499 must clear both rails for every representable base"
        );

        // One step past each edge: the window is tight, not merely sufficient.
        assert!(
            rails(Fixed::from_ratio(9_918_529, 10_000_000)),
            "one step past the positive edge must rail, or the stated window is loose"
        );
        assert!(
            rails(Fixed::ZERO - Fixed::from_ratio(9_687_500, 10_000_000)),
            "one step past the negative edge must rail, or the stated window is loose"
        );

        // The exponents the engine calls inside the window, named so a change to one is caught here.
        for (name, num, den) in [
            ("cube root", 1i64, 3i64),
            ("two thirds", 2, 3),
            ("fourth root", 1, 4),
            ("square root", 1, 2),
            ("Chen-Tse hardness", 585, 1000),
            ("Watson latent heat", 38, 100),
            ("Neufeld collision", 1561, 10000),
            ("Hofmeister simple lattice", 95, 100),
            ("mass-radius beta", 8, 10),
        ] {
            let y = Fixed::from_ratio(num, den);
            assert!(!rails(y), "{name} ({num}/{den}) must be inside the window");
        }
        for (name, num, den) in [
            ("metallicity-luminosity lambda", 44i64, 100i64),
            ("metallicity-radius mu", 18, 1000),
            ("crater outer exponent", 64706, 100000),
        ] {
            let y = Fixed::ZERO - Fixed::from_ratio(num, den);
            assert!(!rails(y), "{name} (-{num}/{den}) must be inside the window");
        }
    }

    /// THE TWO RAILS ARE NOT THE SAME KIND OF FAILURE, measured so the audit's classification stands on a
    /// number. The upper rail's error is unbounded; the lower rail's is about one ulp, because it fires
    /// only where the true value was already at the bottom of the grid.
    #[test]
    fn the_lower_rail_costs_about_one_ulp_and_the_upper_one_is_unbounded() {
        // The zero rail fires below `e^-22`; one ulp is `2^-32`. The gap is one grid step, so a zeroed
        // result was never more than that far from zero.
        let one_ulp = Fixed::from_bits(1).to_f64_lossy();
        let zero_rail = (-22.0f64).exp();
        assert!(
            zero_rail / one_ulp < 1.25,
            "the zero rail must sit within about one ulp of the grid floor, got {} ulp",
            zero_rail / one_ulp
        );
        // The upper rail, by contrast, understates without bound.
        let exponent = Fixed::from_ratio(25, 10);
        let got = Fixed::from_int(10).powi(8).powf(exponent).to_f64_lossy();
        assert_eq!(got, Fixed::MAX.to_f64_lossy(), "(1e8)^2.5 rails");
        assert!(
            1e20 / got > 1e10,
            "the upper rail understates by more than ten orders here, got {}x",
            1e20 / got
        );
    }

    /// A non-positive base is a domain violation, not a result. The bare form returns zero, which a
    /// caller can mistake for a computed value; the checked form refuses.
    #[test]
    fn a_non_positive_base_refuses_rather_than_reading_zero() {
        assert_eq!(Fixed::ZERO.powf(Fixed::ONE), Fixed::ZERO);
        assert!(Fixed::ZERO.checked_powf(Fixed::ONE).is_none());
        assert!((Fixed::ZERO - Fixed::ONE)
            .checked_powf(Fixed::ONE)
            .is_none());
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

    #[test]
    fn the_checked_power_catches_a_wrapping_product_that_the_sentinel_could_not() {
        // THE DEFECT THIS PINS. `powf` is `exp(y ln x)`, and the first checked form read
        // `y.mul(self.ln()).exp()` and compared the RESULT against the rails. `mul` wraps. When `y ln x`
        // itself left the window it wrapped to a plausible argument, `exp` returned a plausible finite
        // number, and no sentinel survived for the check to find. Found by review.
        //
        // A large base with a large exponent is the shape that does it: `ln` is bounded by about 21.49, so a
        // big enough `y` overflows the product long before `exp` is reached.
        let big = Fixed::from_int(1_000_000);
        let huge_y = Fixed::from_int(2_000_000_000);
        assert_eq!(
            big.powf_outcome(huge_y),
            ExpOutcome::Overflow,
            "an overflowing product is caught where it happens, not laundered through exp"
        );
        assert_eq!(big.checked_powf(huge_y), None);

        // THE TWO RAILS ARE DISTINGUISHED, which a single `None` could not do.
        let tiny = Fixed::from_ratio(1, 1000);
        assert_eq!(
            tiny.powf_outcome(Fixed::from_int(100)),
            ExpOutcome::Underflow
        );
        assert_eq!(
            Fixed::from_int(1000).powf_outcome(Fixed::from_int(100)),
            ExpOutcome::Overflow
        );
        // A NON-POSITIVE BASE IS A DOMAIN ERROR, not a rail.
        assert_eq!(Fixed::ZERO.powf_outcome(Fixed::ONE), ExpOutcome::Domain);
        assert_eq!(
            Fixed::from_int(-2).powf_outcome(Fixed::ONE),
            ExpOutcome::Domain
        );

        // AND THE ORDINARY CASE IS UNCHANGED, to the bit, so this is a classification and not a rewrite.
        for (b, e) in [(2i32, 3i32), (10, 2), (7, 1), (3, 4)] {
            let base = Fixed::from_int(b);
            let expo = Fixed::from_int(e);
            assert_eq!(
                base.powf_outcome(expo).finite().map(|v| v.to_bits()),
                Some(base.powf(expo).to_bits()),
                "{b}^{e} must be bit-identical to the bare form"
            );
        }
    }

    #[test]
    fn the_exponential_outcome_names_which_rail_it_hit() {
        assert_eq!(
            Fixed::from_int(30).exp_outcome(),
            ExpOutcome::Overflow,
            "past the upper rail the error is unbounded"
        );
        assert_eq!(
            Fixed::from_int(-30).exp_outcome(),
            ExpOutcome::Underflow,
            "past the lower rail the error is about one ulp"
        );
        assert_eq!(
            Fixed::ONE.exp_outcome().finite().map(|v| v.to_bits()),
            Some(Fixed::ONE.exp().to_bits())
        );
    }

    /// THE EDGE IS THE REPRESENTATION'S, NOT AN AUTHORED 22, pinned by measurement at the crossover.
    ///
    /// The first classified form tested `arg > 22`, copied from [`Fixed::exp`]'s own saturation guard.
    /// `exp` reaches the rail at `ln(Fixed::MAX) = 21.4875626`, so across `(21.4875626, 22]` the argument
    /// test called a saturated `Fixed::MAX` a finite answer. This pins the derived edge, the band that was
    /// laundering, and the fact that `exp` saturates rather than wrapping (which is what makes a comparison
    /// on the argument sound in the first place).
    #[test]
    fn the_representable_edge_is_derived_and_the_authored_band_no_longer_launders() {
        let edge = Fixed::exp_max_argument();

        // The edge is where the algebra says, and the measurement agrees: the last argument below the rail
        // is 21.487560871 and the first at it is 21.487563870, so the derived edge falls between them.
        assert!(
            edge > Fixed::from_ratio(21_487_560, 1_000_000)
                && edge < Fixed::from_ratio(21_487_564, 1_000_000),
            "the derived edge {} must land inside the measured crossover",
            edge.to_f64_lossy()
        );

        // EVERY ARGUMENT IN THE OLD BAND RAILS, and every one of them is now named an overflow rather than
        // returned as a value. Walking it is what proves the band is closed rather than its endpoints moved.
        let mut arg = edge + Fixed::from_ratio(1, 1_000_000);
        let stop = Fixed::from_int(22);
        let step = (stop - edge).div(Fixed::from_int(64));
        while arg <= stop {
            assert_eq!(
                arg.exp().to_bits(),
                Fixed::MAX.to_bits(),
                "exp({}) is a saturated rail",
                arg.to_f64_lossy()
            );
            assert_eq!(
                arg.exp_outcome(),
                ExpOutcome::Overflow,
                "a saturated rail at {} must be named, not returned",
                arg.to_f64_lossy()
            );
            arg += step;
        }

        // BELOW THE EDGE IT STILL ANSWERS, so this closed a hole rather than narrowing the window.
        let inside = edge - Fixed::from_ratio(1, 1000);
        assert!(
            inside.exp() < Fixed::MAX,
            "just inside the edge the exponential is below the rail"
        );
        assert!(
            matches!(inside.exp_outcome(), ExpOutcome::Finite(_)),
            "just inside the edge the answer is finite"
        );

        // THE CASE THAT SURFACED IT: the mass-luminosity power at the production exponent. 463 solar masses
        // resolves and 464 refuses, and 464 is inside the old authored band rather than past 22.
        let alpha = Fixed::from_ratio(35, 10);
        let at_463 = Fixed::from_int(463).powf_outcome(alpha);
        let at_464 = Fixed::from_int(464).powf_outcome(alpha);
        assert!(
            matches!(at_463, ExpOutcome::Finite(_)),
            "463 solar masses is the last representable mass"
        );
        assert_eq!(
            at_464,
            ExpOutcome::Overflow,
            "464 solar masses rails and must be named, not returned as the rail"
        );
        let arg_464 = alpha.mul(Fixed::from_int(464).ln());
        assert!(
            arg_464 > edge && arg_464 < Fixed::from_int(22),
            "464 lands inside the band the authored 22 was letting through"
        );
    }
}
