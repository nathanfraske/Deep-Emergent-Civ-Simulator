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

//! Tier-2 scaled-integer arithmetic (R-UNITS-PIN): a scale-closed `+`, `-`, `*`, `/` on
//! `(i64 mantissa, scale)` pairs, the shape [`crate::AbsoluteQuantity`] already carries. Each operation
//! evaluates in a wide integer intermediate and rounds ONCE per result to a target scale (round-half-to-even
//! through [`crate::idiv_round_half_even`], the same rule the rescale and the composite compute use), so the
//! representation channel contributes only a bounded, checked rounding error and never moves the value
//! (the value-authoring line the framing panel drew for the composite compute).
//!
//! Determinism: integer arithmetic only, no hardware float, so a result is bit-identical on every machine.
//! The result scale is fixed by the load-time scale planner (a later slice), never chosen at runtime, so each
//! operation is a fixed deterministic integer function of its inputs.
//!
//! Intermediate width, sized PER-LAW by measurement (the gate's hardware validation, not `i128` by default):
//! a single multiply of two `i64` mantissas fits `i128` (at most 126 magnitude bits), but a CHAIN rounded
//! ONCE at the end needs far more. The flagship radiant law `sigma * T^4` reaches about a 210-bit accumulator
//! at the planner's derived scales, which exceeds `i128`'s 127-bit signed magnitude, so i256 is a FIRST-CLASS
//! width for chained laws, not an edge case; the quarter-power divide sits about five bits inside `i128` and
//! tips over on a wider envelope. The design choice, made explicit: single-round-PER-CHAIN (evaluate the whole
//! chain in a wide accumulator and round once at the end, maximum precision and exact to the arbitrary-precision
//! oracle, the default for the quartics because the measurement showed it exact) over round-per-OPERATION
//! (round each intermediate to stay in `i128` at a small stated precision loss). The load-time planner (a later
//! slice) sizes each law's accumulator from its measured exponent interval. This slice builds both: the
//! `i128` single-op path (a result that would exceed `i128` returns `None`), and the wide [`WideAccum`], a
//! single-round chain over the eight-sub-limb [`I256`] so the flagship `sigma * T^4` computes at full
//! precision, both proven exact against the arbitrary-precision `BigRat` oracle.

use crate::idiv_round_half_even;
use std::cmp::Ordering;

/// Round `value` to the nearest multiple of `2^shift` and divide it out, ties to even. For `shift == 0`
/// this is the identity. `value` may be negative; the euclidean rounding in [`idiv_round_half_even`] carries
/// the sign correctly. For `shift >= 127` the divisor `2^shift` is not a positive `i128` (`1i128 << 127` sets
/// the sign bit, `>= 128` overflows and panics under the release `overflow-checks`), so the rounding is
/// computed directly on the magnitude: `|value| <= 2^127`, so the quotient magnitude is 0 or 1 and a compare
/// against the half-divisor `2^(shift-1)` decides it (ties to the even 0). This is the exact round-half-even
/// value, matching the wide path and the `BigRat` oracle rather than declining, so a result that underflows
/// the target scale rounds to the correct small mantissa instead of crashing.
fn round_half_even_shr(value: i128, shift: u32) -> i128 {
    if shift == 0 {
        return value;
    }
    if shift < 127 {
        return idiv_round_half_even(value, 1i128 << shift);
    }
    let mag = value.unsigned_abs();
    let half_exp = shift - 1; // >= 126
    let rounded: i128 = if half_exp >= 128 {
        // 2^(shift-1) exceeds u128, so |value| is always below the half-divisor: rounds to zero.
        0
    } else {
        let half = 1u128 << half_exp; // half_exp in [126, 127], fits u128
        match mag.cmp(&half) {
            Ordering::Greater => 1,
            // Less, or an exact half tie (rounds to the even quotient, 0).
            _ => 0,
        }
    };
    if value < 0 {
        -rounded
    } else {
        rounded
    }
}

/// Fit an `i128` result into the `i64` mantissa range, or `None` if it does not (the planner's signal that
/// the result scale is too coarse for the value, or that the intermediate overflowed).
fn fit_i64(value: i128) -> Option<i64> {
    if (i64::MIN as i128..=i64::MAX as i128).contains(&value) {
        Some(value as i64)
    } else {
        None
    }
}

/// Multiply two scaled mantissas to a target scale, rounded ONCE. `a` at scale `s_a` times `b` at scale
/// `s_b`, delivered at scale `s_r`: `round_half_even(a*b / 2^(s_a + s_b - s_r))`. The product of two `i64`
/// mantissas fits `i128`; a result that does not fit `i64` (or a negative net shift whose left shift would
/// overflow `i128`) returns `None`, the signal to widen.
pub fn mul(a: i64, s_a: u32, b: i64, s_b: u32, s_r: u32) -> Option<i64> {
    let product = (a as i128) * (b as i128);
    let net = s_a as i64 + s_b as i64 - s_r as i64;
    let scaled = if net >= 0 {
        round_half_even_shr(product, net as u32)
    } else {
        // The result scale is finer than the inputs': an exact left shift, checked for i128 overflow.
        product.checked_shl((-net) as u32).filter(|v| {
            // checked_shl only guards the shift amount, not value overflow; verify the shift is exact.
            (v >> (-net) as u32) == product
        })?
    };
    fit_i64(scaled)
}

/// Divide two scaled mantissas to a target scale, rounded ONCE. `a` at scale `s_a` over `b` at scale `s_b`,
/// delivered at scale `s_r`: `round_half_even(a * 2^(s_b + s_r - s_a) / b)`. Returns `None` on a zero
/// divisor, on an intermediate that exceeds `i128`, or on a result that does not fit `i64` (widen signal).
pub fn div(a: i64, s_a: u32, b: i64, s_b: u32, s_r: u32) -> Option<i64> {
    if b == 0 {
        return None;
    }
    let neg = (a < 0) ^ (b < 0);
    let mut num = (a as i128).unsigned_abs(); // fits u128, well within range for an i64 magnitude
    let mut den = (b as i128).unsigned_abs();
    let shift = s_b as i64 + s_r as i64 - s_a as i64;
    if shift >= 0 {
        num = num
            .checked_shl(shift as u32)
            .filter(|v| (v >> shift as u32) == num)?;
    } else {
        den = den
            .checked_shl((-shift) as u32)
            .filter(|v| (v >> (-shift) as u32) == den)?;
    }
    // round-half-to-even of num/den with both positive, then reapply the sign. The shift-aligned numerator or
    // denominator can land in [2^127, 2^128), fitting u128 but not signed i128; a raw `as i128` cast would wrap
    // negative and corrupt the divide, so fail loud to None (the widen signal) when either does not fit i128.
    let num_i = i128::try_from(num).ok()?;
    let den_i = i128::try_from(den).ok()?;
    let q = idiv_round_half_even(num_i, den_i);
    let signed = if neg { -q } else { q };
    fit_i64(signed)
}

/// Add two scaled mantissas to a target scale, rounded ONCE. Both are brought to a common (finer) scale by
/// shifting the coarser mantissa up exactly, summed exactly in `i128`, then rounded once to `s_r`. Disparate
/// magnitudes drop the negligible term at the output ULP, the correct rounded answer. Returns `None` if the
/// exact sum or the final result exceeds the representable range (widen signal).
pub fn add(a: i64, s_a: u32, b: i64, s_b: u32, s_r: u32) -> Option<i64> {
    sum_signed(a, s_a, b, s_b, s_r, false)
}

/// Subtract `b` from `a`, otherwise identical to [`add`].
pub fn sub(a: i64, s_a: u32, b: i64, s_b: u32, s_r: u32) -> Option<i64> {
    sum_signed(a, s_a, b, s_b, s_r, true)
}

/// Floor of the integer square root of a non-negative `u128`, by the classic digit-by-digit method (no
/// division, deterministic, bit-identical everywhere).
fn floor_isqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    // Start `bit` at the largest even power of two not exceeding `n` (a power of four).
    let mut bit: u128 = 1u128 << ((127 - n.leading_zeros()) & !1);
    let mut num = n;
    let mut res: u128 = 0;
    while bit != 0 {
        if num >= res + bit {
            num -= res + bit;
            res = (res >> 1) + bit;
        } else {
            res >>= 1;
        }
        bit >>= 2;
    }
    res
}

/// Scale-aware square root, rounded to nearest: the square root of `bits` at scale `s_in`, delivered at scale
/// `s_out`. Since `sqrt(bits/2^s_in) * 2^s_out = sqrt(bits * 2^(2*s_out - s_in))`, it is one integer square
/// root over a shifted argument. The quarter-power consumer `(P/(c*K))^(1/4)` is two of these, avoiding a
/// transcendental. Requires `bits >= 0`; returns `None` on a negative argument, a non-negative-shift the
/// planner did not provide (result scale too coarse, widen `s_out`), or an argument that exceeds the
/// intermediate (widen signal).
pub fn isqrt(bits: i64, s_in: u32, s_out: u32) -> Option<i64> {
    if bits < 0 {
        return None;
    }
    let shift = 2 * s_out as i64 - s_in as i64;
    if shift < 0 {
        // The result scale is too coarse to hold the root at integer precision; the planner picks a finer
        // s_out. Signalled rather than silently truncated.
        return None;
    }
    let arg = (bits as u128)
        .checked_shl(shift as u32)
        .filter(|v| (v >> shift as u32) == bits as u128)?;
    let r = floor_isqrt(arg);
    // Round to nearest: step up when the argument is past the midpoint r^2 + r (no exact tie occurs for an
    // integer argument and a half-integer root).
    let rounded = if arg - r * r > r { r + 1 } else { r };
    fit_i64(rounded as i128)
}

fn sum_signed(a: i64, s_a: u32, b: i64, s_b: u32, s_r: u32, subtract: bool) -> Option<i64> {
    let common = s_a.max(s_b);
    // Shift the coarser mantissa up to the common (finer) scale, exact; the finer one's shift is zero.
    let a_common = (a as i128)
        .checked_shl(common - s_a)
        .filter(|v| (v >> (common - s_a)) == a as i128)?;
    let b_raw = (b as i128)
        .checked_shl(common - s_b)
        .filter(|v| (v >> (common - s_b)) == b as i128)?;
    let b_common = if subtract { -b_raw } else { b_raw };
    let sum = a_common.checked_add(b_common)?;
    let scaled = if common >= s_r {
        round_half_even_shr(sum, common - s_r)
    } else {
        sum.checked_shl(s_r - common)
            .filter(|v| (v >> (s_r - common)) == sum)?
    };
    fit_i64(scaled)
}

// ---- the wide (i256) intermediate and the single-round chain evaluator ----

/// A signed 256-bit integer: a sign and four little-endian `u64` magnitude limbs. This is the eight-sub-limb
/// wide intermediate the gate's hardware validation showed the flagship `sigma * T^4` needs (its single-round
/// accumulator reaches about 210 bits, over `i128`). An operation that would exceed 256 magnitude bits returns
/// `None`, the signal to widen further or replan the scales. Integer-only, so bit-identical on every machine.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct I256 {
    neg: bool,
    mag: [u64; 4],
}

impl I256 {
    fn zero() -> Self {
        I256 {
            neg: false,
            mag: [0; 4],
        }
    }

    fn from_i64(v: i64) -> Self {
        I256 {
            neg: v < 0,
            mag: [v.unsigned_abs(), 0, 0, 0],
        }
    }

    fn is_zero(&self) -> bool {
        self.mag == [0u64; 4]
    }

    fn cmp_mag(a: &[u64; 4], b: &[u64; 4]) -> Ordering {
        for i in (0..4).rev() {
            if a[i] != b[i] {
                return a[i].cmp(&b[i]);
            }
        }
        Ordering::Equal
    }

    /// Multiply by a signed `i64`, `None` on 256-bit overflow. Schoolbook multiply of the magnitude by a
    /// single `u64` limb.
    fn mul_i64(&self, v: i64) -> Option<I256> {
        if v == 0 || self.is_zero() {
            return Some(I256::zero());
        }
        let m = v.unsigned_abs();
        let mut out = [0u64; 4];
        let mut carry: u128 = 0;
        for (i, out_limb) in out.iter_mut().enumerate() {
            let cur = (self.mag[i] as u128) * (m as u128) + carry;
            *out_limb = cur as u64;
            carry = cur >> 64;
        }
        if carry != 0 {
            return None;
        }
        Some(I256 {
            neg: self.neg ^ (v < 0),
            mag: out,
        })
    }

    /// Signed sum, `None` on 256-bit overflow.
    fn add(&self, other: &I256) -> Option<I256> {
        if self.neg == other.neg {
            let mut out = [0u64; 4];
            let mut carry: u128 = 0;
            for (i, out_limb) in out.iter_mut().enumerate() {
                let cur = self.mag[i] as u128 + other.mag[i] as u128 + carry;
                *out_limb = cur as u64;
                carry = cur >> 64;
            }
            if carry != 0 {
                return None;
            }
            Some(I256 {
                neg: self.neg,
                mag: out,
            })
        } else {
            let (neg, big, small) = match I256::cmp_mag(&self.mag, &other.mag) {
                Ordering::Equal => return Some(I256::zero()),
                Ordering::Greater => (self.neg, &self.mag, &other.mag),
                Ordering::Less => (other.neg, &other.mag, &self.mag),
            };
            let mut out = [0u64; 4];
            let mut borrow: i128 = 0;
            for (i, out_limb) in out.iter_mut().enumerate() {
                let cur = big[i] as i128 - small[i] as i128 - borrow;
                if cur < 0 {
                    *out_limb = (cur + (1i128 << 64)) as u64;
                    borrow = 1;
                } else {
                    *out_limb = cur as u64;
                    borrow = 0;
                }
            }
            Some(I256 { neg, mag: out })
        }
    }

    fn negate(&self) -> I256 {
        if self.is_zero() {
            *self
        } else {
            I256 {
                neg: !self.neg,
                mag: self.mag,
            }
        }
    }

    fn sub(&self, other: &I256) -> Option<I256> {
        self.add(&other.negate())
    }

    fn test_bit(mag: &[u64; 4], i: u32) -> bool {
        (mag[(i / 64) as usize] >> (i % 64)) & 1 == 1
    }

    fn shr_mag(mag: &[u64; 4], shift: u32) -> [u64; 4] {
        let limb = (shift / 64) as usize;
        let bit = shift % 64;
        let mut out = [0u64; 4];
        let mut i = 0;
        while i + limb < 4 {
            let mut v = mag[i + limb] >> bit;
            if bit > 0 && i + limb + 1 < 4 {
                v |= mag[i + limb + 1] << (64 - bit);
            }
            out[i] = v;
            i += 1;
        }
        out
    }

    fn any_bit_below(mag: &[u64; 4], bits: u32) -> bool {
        (0..bits).any(|i| I256::test_bit(mag, i))
    }

    /// Round the value right by `shift` bits, ties to even, and return the signed magnitude as an `i128`
    /// (the mantissa fits `i128`), or `None` if the rounded magnitude exceeds `i128`.
    fn round_shr(&self, shift: u32) -> Option<i128> {
        let mut q = I256::shr_mag(&self.mag, shift);
        if shift > 0 {
            let round_bit = I256::test_bit(&self.mag, shift - 1);
            if round_bit {
                let sticky = I256::any_bit_below(&self.mag, shift - 1);
                let q_odd = q[0] & 1 == 1;
                if sticky || q_odd {
                    // q += 1
                    let mut carry = 1u128;
                    for limb in q.iter_mut() {
                        let cur = *limb as u128 + carry;
                        *limb = cur as u64;
                        carry = cur >> 64;
                        if carry == 0 {
                            break;
                        }
                    }
                }
            }
        }
        if q[2] != 0 || q[3] != 0 {
            return None;
        }
        let mag = (q[0] as u128) | ((q[1] as u128) << 64);
        if mag > i128::MAX as u128 {
            return None;
        }
        Some(if self.neg {
            -(mag as i128)
        } else {
            mag as i128
        })
    }
}

/// A single-round chain accumulator: a wide (`i256`) value at a running scale. Multiply scaled mantissas in,
/// combine sub-chains with `sub` (the difference-of-quartics), then round ONCE to the output scale. This is
/// the single-round-per-chain form the gate's measurement showed exact to the arbitrary-precision oracle:
/// `k` and every factor enter at full scale, the chain accumulates in the wide intermediate, and the only
/// rounding is the single terminal one.
#[derive(Clone, Copy, Debug)]
pub struct WideAccum {
    value: I256,
    scale: u32,
}

impl WideAccum {
    /// Start a chain from a scaled mantissa.
    pub fn new(bits: i64, scale: u32) -> Self {
        WideAccum {
            value: I256::from_i64(bits),
            scale,
        }
    }

    /// Multiply another scaled mantissa into the chain; the running scale accumulates. `None` on i256 overflow.
    pub fn mul(&self, bits: i64, scale: u32) -> Option<WideAccum> {
        Some(WideAccum {
            value: self.value.mul_i64(bits)?,
            scale: self.scale + scale,
        })
    }

    /// The integer power of a single scaled mantissa: `bits^exp` at scale `exp*scale`, by repeated multiply in
    /// the wide accumulator. `exp == 0` is the dimensionless one at scale zero.
    pub fn power(bits: i64, scale: u32, exp: u32) -> Option<WideAccum> {
        if exp == 0 {
            return Some(WideAccum::new(1, 0));
        }
        let mut acc = WideAccum::new(bits, scale);
        for _ in 1..exp {
            acc = acc.mul(bits, scale)?;
        }
        Some(acc)
    }

    /// Subtract another chain AT THE SAME running scale (the difference-of-quartics case). `None` if the scales
    /// differ (the planner keeps the two sub-chains at one scale) or on overflow.
    pub fn sub(&self, other: &WideAccum) -> Option<WideAccum> {
        if self.scale != other.scale {
            return None;
        }
        Some(WideAccum {
            value: self.value.sub(&other.value)?,
            scale: self.scale,
        })
    }

    /// Add another chain at the same running scale. `None` if the scales differ or on overflow.
    pub fn add(&self, other: &WideAccum) -> Option<WideAccum> {
        if self.scale != other.scale {
            return None;
        }
        Some(WideAccum {
            value: self.value.add(&other.value)?,
            scale: self.scale,
        })
    }

    /// Round the accumulated chain ONCE to the output scale, ties to even, returning the `i64` mantissa. The
    /// output scale must not exceed the running scale (the chain is finer than its result); `None` otherwise,
    /// or if the rounded mantissa does not fit `i64`.
    pub fn round_to_scale(&self, target: u32) -> Option<i64> {
        if target > self.scale {
            return None;
        }
        fit_i64(self.value.round_shr(self.scale - target)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bignum::{BigRat, BigUint};
    use std::cmp::Ordering;

    /// The exact value of a scaled mantissa as a rational: `bits / 2^scale`.
    fn as_rat(bits: i64, scale: u32) -> BigRat {
        BigRat::new(
            bits < 0,
            BigUint::from_u64(bits.unsigned_abs()),
            BigUint::from_u64(1).shl_bits(scale),
        )
    }

    /// The oracle: the exact rational result rounded ONCE to the target scale, as the i128 the op must return.
    fn oracle(exact: &BigRat, s_r: u32) -> i64 {
        exact.round_to_scale(s_r).unwrap() as i64
    }

    #[test]
    fn mul_matches_the_exact_rational_oracle() {
        let cases = [
            (3i64, 4u32, 5i64, 4u32, 4u32),
            (1_000_000, 20, 7, 8, 16),
            (-123_456, 12, 654_321, 10, 14),
            (2, 32, 5, 32, 32),
            (7, 8, 9, 8, 4), // coarser result: rounding happens
        ];
        for (a, sa, b, sb, sr) in cases {
            let got = mul(a, sa, b, sb, sr).unwrap();
            let want = oracle(&as_rat(a, sa).mul(&as_rat(b, sb)), sr);
            assert_eq!(got, want, "mul({a}@{sa} * {b}@{sb} -> {sr})");
        }
    }

    #[test]
    fn div_matches_the_exact_rational_oracle() {
        let cases = [
            (10i64, 4u32, 4i64, 4u32, 8u32),
            (1, 8, 3, 8, 20),
            (-2, 8, 3, 8, 20),
            (5_670_374, 40, 3, 0, 40),
            (1_000, 10, 7, 10, 10),
        ];
        for (a, sa, b, sb, sr) in cases {
            let got = div(a, sa, b, sb, sr).unwrap();
            let want = oracle(&as_rat(a, sa).div(&as_rat(b, sb)), sr);
            assert_eq!(got, want, "div({a}@{sa} / {b}@{sb} -> {sr})");
        }
    }

    #[test]
    fn add_and_sub_match_the_exact_rational_oracle_including_disparate_magnitudes() {
        let cases = [
            (3i64, 4u32, 5i64, 4u32, 4u32),
            (1, 0, 1, 40, 40), // a big and a tiny term at very different scales
            (1_000_000, 20, -3, 20, 20),
            (7, 8, 9, 12, 10),
        ];
        for (a, sa, b, sb, sr) in cases {
            let got_add = add(a, sa, b, sb, sr).unwrap();
            let want_add = oracle(&as_rat(a, sa).add(&as_rat(b, sb)), sr);
            assert_eq!(got_add, want_add, "add({a}@{sa} + {b}@{sb} -> {sr})");
            let got_sub = sub(a, sa, b, sb, sr).unwrap();
            let want_sub = oracle(&as_rat(a, sa).sub(&as_rat(b, sb)), sr);
            assert_eq!(got_sub, want_sub, "sub({a}@{sa} - {b}@{sb} -> {sr})");
        }
    }

    #[test]
    fn multiply_is_commutative_at_the_bit_level() {
        // The single terminal rounding makes the op a fixed function of its inputs: a*b == b*a bit-for-bit.
        for (a, sa, b, sb, sr) in [(123i64, 10u32, 456i64, 7u32, 12u32), (-9, 8, 17, 5, 6)] {
            assert_eq!(mul(a, sa, b, sb, sr), mul(b, sb, a, sa, sr));
        }
    }

    #[test]
    fn a_tie_rounds_to_even() {
        // 3/2 at scale 0 is exactly 1.5, ties to even -> 2; 1/2 -> 0.
        assert_eq!(div(3, 0, 2, 0, 0), Some(2));
        assert_eq!(div(1, 0, 2, 0, 0), Some(0));
        // mul: (1@1)*(1@1) = 0.25 to scale 0 is 0; (3@1)*(1@0)=1.5 to scale 0 -> 2 (even).
        assert_eq!(mul(3, 1, 1, 0, 0), Some(2));
    }

    #[test]
    fn a_large_scale_delta_underflows_to_zero_without_panicking() {
        // The gate's fuzz found round_half_even_shr panicked (release overflow-checks) or miscomputed at a
        // scale delta of 127+, where 1i128 << shift is not a positive i128. Both must now round exactly.
        // A net shift of 200 (s_a+s_b-s_r) underflows the result scale: the exact value is far below one ULP,
        // so it rounds to zero rather than crashing.
        assert_eq!(mul(1, 100, 1, 100, 0), Some(0));
        // The gate's exact-verified shift==127 case: the old code read 1i128 << 127 as a negative divisor and
        // returned Some(1); the exact round-half-even value is 0.
        assert_eq!(
            mul(5315962130996935763, 92, 2919668674121976043, 102, 67),
            Some(0)
        );
        // add/sub share the helper: a coarse target scale with a 128+ net shift rounds, does not panic.
        assert_eq!(add(1, 130, 1, 130, 0), Some(0));
        assert_eq!(sub(1, 130, 1, 130, 0), Some(0));
    }

    #[test]
    fn a_divide_whose_aligned_numerator_exceeds_signed_i128_declines_rather_than_wrapping() {
        // The gate's exact-verified case: the shift-aligned numerator lands in [2^127, 2^128), fitting u128 but
        // not signed i128. The old `num as i128` wrapped negative and returned Some(-2559781616330884959); the
        // documented contract is None (the widen signal).
        assert_eq!(
            div(4403335111641285598, 27, 6005818356516761817, 61, 32),
            None
        );
    }

    #[test]
    fn oracle_agreement_over_a_swept_grid() {
        // A deterministic sweep (no RNG): every combination agrees with the exact-rational oracle.
        let mants = [1i64, 2, 3, 7, 255, -13, -1000, 65_537];
        let scales = [0u32, 4, 12, 24, 32];
        for &a in &mants {
            for &b in &mants {
                for &sa in &scales {
                    for &sb in &scales {
                        let sr = 20u32;
                        if let Some(got) = mul(a, sa, b, sb, sr) {
                            assert_eq!(got, oracle(&as_rat(a, sa).mul(&as_rat(b, sb)), sr));
                        }
                        if b != 0 {
                            if let Some(got) = div(a, sa, b, sb, sr) {
                                assert_eq!(got, oracle(&as_rat(a, sa).div(&as_rat(b, sb)), sr));
                            }
                        }
                        if let Some(got) = add(a, sa, b, sb, sr) {
                            assert_eq!(got, oracle(&as_rat(a, sa).add(&as_rat(b, sb)), sr));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn floor_isqrt_brackets_the_root() {
        for n in [
            0u128,
            1,
            2,
            3,
            4,
            8,
            15,
            16,
            17,
            99,
            100,
            1_000_000,
            u128::from(u64::MAX),
        ] {
            let r = floor_isqrt(n);
            assert!(r * r <= n, "floor_isqrt({n}) = {r}, r^2 > n");
            assert!(
                (r + 1).checked_mul(r + 1).is_none_or(|sq| sq > n),
                "floor_isqrt({n}) too small"
            );
        }
    }

    #[test]
    fn isqrt_is_scale_aware_and_rounds_to_nearest() {
        // sqrt(4) = 2, at any equal in/out scale.
        assert_eq!(isqrt(4 << 20, 20, 20), Some(2 << 20));
        // sqrt(2) at scale 32: round(1.41421356 * 2^32) = 6074001000.5 -> compute the oracle by hand below.
        // Oracle: for value = bits/2^s_in, expected = nearest integer to sqrt(value) * 2^s_out
        //       = nearest integer to sqrt(bits * 2^(2*s_out - s_in)).
        let oracle = |bits: i64, s_in: u32, s_out: u32| -> i64 {
            let arg = (bits as u128) << (2 * s_out - s_in);
            let r = floor_isqrt(arg);
            (if arg - r * r > r { r + 1 } else { r }) as i64
        };
        for (bits, s_in, s_out) in [
            (2i64, 0u32, 16u32),
            (5_670_374, 20, 24),
            (1, 0, 30),
            (123_456_789, 12, 20),
        ] {
            assert_eq!(
                isqrt(bits, s_in, s_out),
                Some(oracle(bits, s_in, s_out)),
                "isqrt({bits}@{s_in} -> {s_out})"
            );
        }
        // A negative argument or a too-coarse result scale is a widen signal, not a wrong value.
        assert_eq!(isqrt(-1, 0, 0), None);
        assert_eq!(isqrt(4, 40, 0), None); // 2*0 - 40 < 0
    }

    #[test]
    fn two_isqrts_make_a_quarter_power() {
        // (16)^(1/4) = 2: sqrt(sqrt(16)). Carry a fine scale through the intermediate.
        let s = 24u32;
        let root2 = isqrt(16 << s, s, s).unwrap(); // sqrt(16) = 4 at scale s
        let root4 = isqrt(root2, s, s).unwrap(); // sqrt(4) = 2 at scale s
        assert_eq!(root4, 2 << s);
    }

    /// The exact rational of a product-of-powers chain, for the wide-accumulator oracle.
    fn chain_rat(factors: &[(i64, u32, u32)]) -> BigRat {
        let mut acc = BigRat::from_i64(1);
        for &(bits, scale, exp) in factors {
            let f = as_rat(bits, scale);
            for _ in 0..exp {
                acc = acc.mul(&f);
            }
        }
        acc
    }

    #[test]
    fn wide_chain_sigma_t4_matches_the_exact_oracle() {
        // The flagship radiant chain sigma * T^4 in the single-round wide accumulator, versus the exact
        // rational rounded once. sigma near its scale-55 mantissa, T near 288 K at scale 20.
        let (sigma, s_sigma) = (2_042_913_741i64, 55u32);
        let (t, s_t) = (288i64 << 20, 20u32);
        let s_out = 32u32;
        let chain = WideAccum::power(t, s_t, 4)
            .unwrap()
            .mul(sigma, s_sigma)
            .unwrap();
        let got = chain.round_to_scale(s_out).unwrap();
        let want = chain_rat(&[(sigma, s_sigma, 1), (t, s_t, 4)])
            .round_to_scale(s_out)
            .unwrap() as i64;
        assert_eq!(got, want);
    }

    #[test]
    fn wide_difference_of_quartics_matches_the_exact_oracle() {
        // sigma * (T_hot^4 - T_cold^4): the cancellation is exact in the wide integer, rounded once.
        let (sigma, s_sigma) = (2_042_913_741i64, 55u32);
        let (t_hot, t_cold, s_t) = (310i64 << 18, 280i64 << 18, 18u32);
        let s_out = 30u32;
        let diff = WideAccum::power(t_hot, s_t, 4)
            .unwrap()
            .sub(&WideAccum::power(t_cold, s_t, 4).unwrap())
            .unwrap();
        let got = diff
            .mul(sigma, s_sigma)
            .unwrap()
            .round_to_scale(s_out)
            .unwrap();
        let exact = chain_rat(&[(t_hot, s_t, 4)])
            .sub(&chain_rat(&[(t_cold, s_t, 4)]))
            .mul(&as_rat(sigma, s_sigma));
        assert_eq!(got, exact.round_to_scale(s_out).unwrap() as i64);
    }

    #[test]
    fn wide_chain_swept_grid_matches_the_oracle() {
        // A deterministic sweep of small chains (products of powers), each exact to the rational oracle.
        let mants = [2i64, 3, 5, -7, 129, -4096];
        let scales = [0u32, 8, 20];
        for &a in &mants {
            for &b in &mants {
                for &sa in &scales {
                    for &sb in &scales {
                        let s_out = 16u32;
                        // chain = a@sa * (b@sb)^3
                        if let Some(chain) = WideAccum::new(a, sa)
                            .mul(b, sb)
                            .and_then(|w| w.mul(b, sb))
                            .and_then(|w| w.mul(b, sb))
                        {
                            if let Some(got) = chain.round_to_scale(s_out) {
                                let want = chain_rat(&[(a, sa, 1), (b, sb, 3)])
                                    .round_to_scale(s_out)
                                    .unwrap() as i64;
                                assert_eq!(got, want, "chain {a}@{sa} * ({b}@{sb})^3 -> {s_out}");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn wide_chain_needs_more_than_i128_but_fits_i256() {
        // A quartic that overflows i128 (so the single-op mul path would return None) computes in the wide
        // accumulator: a ~40-bit mantissa to the 4th is ~160 bits, past i128's 127.
        let (t, s_t) = ((1i64 << 40) + 12345, 32u32);
        let chain = WideAccum::power(t, s_t, 4).unwrap();
        let got = chain.round_to_scale(20).unwrap();
        let want = chain_rat(&[(t, s_t, 4)]).round_to_scale(20).unwrap() as i64;
        assert_eq!(got, want);
        // t^2 keeping the full product at scale 2*s_t exceeds i64, so the single-op i128 mul returns the
        // widen signal there (it cannot carry the un-rounded quartic that the wide accumulator holds).
        assert_eq!(mul(t, s_t, t, s_t, 2 * s_t), None);
    }

    // Guard the oracle helper itself against a stale comparison path.
    #[test]
    fn as_rat_round_trips_a_known_value() {
        assert_eq!(
            as_rat(5, 2)
                .cmp_rat(&BigRat::from_i64(1).add(&BigRat::from_i64(1).div(&BigRat::from_i64(4)))),
            Ordering::Equal
        );
    }
}
