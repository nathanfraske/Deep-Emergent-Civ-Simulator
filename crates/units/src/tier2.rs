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
//! Intermediate width: a single multiply of two `i64` mantissas fits `i128` (at most 126 magnitude bits), so
//! the common case is native `i128`. An operation whose intermediate would exceed `i128` (a chain of powers,
//! or a divide whose numerator shift plus mantissa exceeds 127 bits) returns `None` from the `i128` path,
//! the planner's signal to route that node through the wider intermediate (the i256 / bignum path, a later
//! slice sized on the gate's hardware-validation survey). This slice builds the `i128` path and its exact
//! contract; the wide path lands parameterized behind the same interface.

use crate::idiv_round_half_even;

/// Round `value` to the nearest multiple of `2^shift` and divide it out, ties to even. For `shift == 0`
/// this is the identity. `value` may be negative; the euclidean rounding in [`idiv_round_half_even`] carries
/// the sign correctly.
fn round_half_even_shr(value: i128, shift: u32) -> i128 {
    if shift == 0 {
        value
    } else {
        idiv_round_half_even(value, 1i128 << shift)
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
    // round-half-to-even of num/den with both positive, then reapply the sign.
    let q = idiv_round_half_even(num as i128, den as i128);
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
