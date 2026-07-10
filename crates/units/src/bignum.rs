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

//! Deterministic arbitrary-precision integer and exact-rational arithmetic, used by the composite
//! constant compute (R-UNITS-PIN). No floating point appears anywhere: every operation is integer
//! arithmetic over `u32` limbs, so a result is bit-identical on every machine, which is the property the
//! composite derivation needs (a fundamental like the Boltzmann constant underflows the canonical Q32.32
//! epsilon, so the formula is evaluated EXACTLY as a rational and rounded ONCE at the end, and exact
//! evaluation with a single terminal rounding is order-independent by construction).
//!
//! The module is deliberately minimal: it carries only the operations the composite evaluator and the
//! deterministic pi series need (multiply, add, subtract, shift, long division, exponentiation, and a
//! single round-half-to-even to a fixed-point scale), not a general bignum library.

use std::cmp::Ordering;

/// An arbitrary-precision unsigned integer, little-endian base `2^32`. The invariant is that `limbs`
/// carries no trailing (most-significant) zero limb, so a value has one canonical representation and the
/// empty vector is zero. All arithmetic preserves this invariant.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BigUint {
    limbs: Vec<u32>,
}

impl BigUint {
    /// Zero.
    pub fn zero() -> Self {
        BigUint { limbs: Vec::new() }
    }

    /// A single-limb-or-two value from a `u64`.
    pub fn from_u64(v: u64) -> Self {
        let mut b = BigUint {
            limbs: vec![(v & 0xFFFF_FFFF) as u32, (v >> 32) as u32],
        };
        b.trim();
        b
    }

    /// True when the value is zero.
    pub fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    /// Drop trailing zero limbs to restore the canonical form.
    fn trim(&mut self) {
        while let Some(&0) = self.limbs.last() {
            self.limbs.pop();
        }
    }

    /// The number of significant bits (0 for zero).
    pub fn bit_len(&self) -> u32 {
        match self.limbs.last() {
            None => 0,
            Some(&top) => (self.limbs.len() as u32 - 1) * 32 + (32 - top.leading_zeros()),
        }
    }

    /// Bit `i` (0 = least significant).
    fn test_bit(&self, i: u32) -> bool {
        let limb = (i / 32) as usize;
        if limb >= self.limbs.len() {
            return false;
        }
        (self.limbs[limb] >> (i % 32)) & 1 == 1
    }

    /// Ordering by magnitude.
    pub fn cmp_big(&self, other: &BigUint) -> Ordering {
        if self.limbs.len() != other.limbs.len() {
            return self.limbs.len().cmp(&other.limbs.len());
        }
        // Same limb count: compare from the most significant limb down.
        for i in (0..self.limbs.len()).rev() {
            if self.limbs[i] != other.limbs[i] {
                return self.limbs[i].cmp(&other.limbs[i]);
            }
        }
        Ordering::Equal
    }

    /// Sum.
    pub fn add(&self, other: &BigUint) -> BigUint {
        let n = self.limbs.len().max(other.limbs.len());
        let mut out = Vec::with_capacity(n + 1);
        let mut carry: u64 = 0;
        for i in 0..n {
            let a = *self.limbs.get(i).unwrap_or(&0) as u64;
            let b = *other.limbs.get(i).unwrap_or(&0) as u64;
            let s = a + b + carry;
            out.push((s & 0xFFFF_FFFF) as u32);
            carry = s >> 32;
        }
        if carry != 0 {
            out.push(carry as u32);
        }
        let mut r = BigUint { limbs: out };
        r.trim();
        r
    }

    /// Difference `self - other`, which requires `self >= other` (panics otherwise, since the caller
    /// establishes the ordering).
    pub fn sub(&self, other: &BigUint) -> BigUint {
        debug_assert!(self.cmp_big(other) != Ordering::Less, "sub underflow");
        let mut out = Vec::with_capacity(self.limbs.len());
        let mut borrow: i64 = 0;
        for i in 0..self.limbs.len() {
            let a = self.limbs[i] as i64;
            let b = *other.limbs.get(i).unwrap_or(&0) as i64;
            let mut d = a - b - borrow;
            if d < 0 {
                d += 1 << 32;
                borrow = 1;
            } else {
                borrow = 0;
            }
            out.push(d as u32);
        }
        let mut r = BigUint { limbs: out };
        r.trim();
        r
    }

    /// Product (schoolbook; the operands here stay a few hundred limbs at most).
    pub fn mul(&self, other: &BigUint) -> BigUint {
        if self.is_zero() || other.is_zero() {
            return BigUint::zero();
        }
        let mut out = vec![0u32; self.limbs.len() + other.limbs.len()];
        for (i, &a) in self.limbs.iter().enumerate() {
            let mut carry: u64 = 0;
            let a = a as u64;
            for (j, &b) in other.limbs.iter().enumerate() {
                let idx = i + j;
                let cur = out[idx] as u64 + a * (b as u64) + carry;
                out[idx] = (cur & 0xFFFF_FFFF) as u32;
                carry = cur >> 32;
            }
            out[i + other.limbs.len()] += carry as u32;
        }
        let mut r = BigUint { limbs: out };
        r.trim();
        r
    }

    /// Left shift by `bits`, i.e. multiply by `2^bits`.
    pub fn shl_bits(&self, bits: u32) -> BigUint {
        if self.is_zero() {
            return BigUint::zero();
        }
        let limb_shift = (bits / 32) as usize;
        let bit_shift = bits % 32;
        let mut out = vec![0u32; limb_shift];
        if bit_shift == 0 {
            out.extend_from_slice(&self.limbs);
        } else {
            let mut carry: u64 = 0;
            for &limb in &self.limbs {
                let cur = ((limb as u64) << bit_shift) | carry;
                out.push((cur & 0xFFFF_FFFF) as u32);
                carry = cur >> 32;
            }
            if carry != 0 {
                out.push(carry as u32);
            }
        }
        let mut r = BigUint { limbs: out };
        r.trim();
        r
    }

    /// Quotient and remainder `(q, r)` with `self = q * divisor + r`, `0 <= r < divisor`. Bit-by-bit
    /// long division (shift-and-subtract): O(bit_len) subtractions, simple enough to be obviously correct,
    /// and fast enough at these sizes. Panics on a zero divisor.
    pub fn divmod(&self, divisor: &BigUint) -> (BigUint, BigUint) {
        assert!(!divisor.is_zero(), "division by zero");
        if self.cmp_big(divisor) == Ordering::Less {
            return (BigUint::zero(), self.clone());
        }
        let mut quotient = BigUint::zero();
        let mut remainder = BigUint::zero();
        for i in (0..self.bit_len()).rev() {
            // remainder = (remainder << 1) | bit i of self
            remainder = remainder.shl_bits(1);
            if self.test_bit(i) {
                remainder = remainder.add(&BigUint::from_u64(1));
            }
            if remainder.cmp_big(divisor) != Ordering::Less {
                remainder = remainder.sub(divisor);
                quotient.set_bit(i);
            }
        }
        (quotient, remainder)
    }

    /// Set bit `i` to one (used only by `divmod` to build the quotient).
    fn set_bit(&mut self, i: u32) {
        let limb = (i / 32) as usize;
        if limb >= self.limbs.len() {
            self.limbs.resize(limb + 1, 0);
        }
        self.limbs[limb] |= 1 << (i % 32);
    }

    /// `self` raised to `exp` (exponentiation by squaring).
    pub fn pow(&self, exp: u32) -> BigUint {
        let mut result = BigUint::from_u64(1);
        let mut base = self.clone();
        let mut e = exp;
        while e > 0 {
            if e & 1 == 1 {
                result = result.mul(&base);
            }
            e >>= 1;
            if e > 0 {
                base = base.mul(&base);
            }
        }
        result
    }

    /// Ten raised to `exp`.
    pub fn ten_pow(exp: u32) -> BigUint {
        BigUint::from_u64(10).pow(exp)
    }

    /// The value as a `u128` when it fits, else `None` (used to extract a rounded fixed-point magnitude).
    pub fn to_u128(&self) -> Option<u128> {
        if self.limbs.len() > 4 {
            return None;
        }
        let mut v: u128 = 0;
        for (i, &limb) in self.limbs.iter().enumerate() {
            v |= (limb as u128) << (32 * i as u32);
        }
        Some(v)
    }
}

/// An exact signed rational `(-1)^neg * num / den`, with `den` never zero. No reduction to lowest terms
/// is performed: the composite evaluator multiplies a small fixed number of factors and rounds once, so
/// the numerator and denominator stay a few hundred limbs and a gcd pass would only add cost.
#[derive(Clone, Debug)]
pub struct BigRat {
    neg: bool,
    num: BigUint,
    den: BigUint,
}

impl BigRat {
    /// A rational from a signed integer.
    pub fn from_i64(v: i64) -> Self {
        BigRat {
            neg: v < 0,
            num: BigUint::from_u64(v.unsigned_abs()),
            den: BigUint::from_u64(1),
        }
    }

    /// A rational `num / den` (both non-negative magnitudes), sign `neg`.
    pub fn new(neg: bool, num: BigUint, den: BigUint) -> Self {
        assert!(!den.is_zero(), "rational with zero denominator");
        // Zero is canonically non-negative.
        let neg = if num.is_zero() { false } else { neg };
        BigRat { neg, num, den }
    }

    /// True when the value is zero.
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    /// Negation.
    pub fn negate(&self) -> BigRat {
        BigRat::new(!self.neg, self.num.clone(), self.den.clone())
    }

    /// Product.
    pub fn mul(&self, other: &BigRat) -> BigRat {
        BigRat::new(
            self.neg ^ other.neg,
            self.num.mul(&other.num),
            self.den.mul(&other.den),
        )
    }

    /// Quotient.
    pub fn div(&self, other: &BigRat) -> BigRat {
        assert!(!other.num.is_zero(), "rational division by zero");
        BigRat::new(
            self.neg ^ other.neg,
            self.num.mul(&other.den),
            self.den.mul(&other.num),
        )
    }

    /// Sum, over the common denominator `self.den * other.den`.
    pub fn add(&self, other: &BigRat) -> BigRat {
        let a = self.num.mul(&other.den); // |self| numerator over common den
        let b = other.num.mul(&self.den); // |other| numerator over common den
        let den = self.den.mul(&other.den);
        match (self.neg, other.neg) {
            (false, false) => BigRat::new(false, a.add(&b), den),
            (true, true) => BigRat::new(true, a.add(&b), den),
            (false, true) | (true, false) => {
                // Signs differ: subtract the smaller magnitude from the larger and take its sign.
                let (larger_neg, big, small) = if a.cmp_big(&b) != Ordering::Less {
                    (self.neg, &a, &b)
                } else {
                    (other.neg, &b, &a)
                };
                BigRat::new(larger_neg, big.sub(small), den)
            }
        }
    }

    /// Difference.
    pub fn sub(&self, other: &BigRat) -> BigRat {
        self.add(&other.negate())
    }

    /// Ordering of two rationals by value (compares `self.num*other.den` against `other.num*self.den`
    /// with sign).
    pub fn cmp_rat(&self, other: &BigRat) -> Ordering {
        match (self.neg, other.neg) {
            (false, true) => return Ordering::Greater,
            (true, false) => return Ordering::Less,
            _ => {}
        }
        let a = self.num.mul(&other.den);
        let b = other.num.mul(&self.den);
        let mag = a.cmp_big(&b);
        if self.neg {
            mag.reverse()
        } else {
            mag
        }
    }

    /// The absolute value.
    pub fn abs(&self) -> BigRat {
        BigRat::new(false, self.num.clone(), self.den.clone())
    }

    /// Round `self` to a signed integer magnitude at fixed-point scale `scale_bits`, i.e. the nearest
    /// integer to `self * 2^scale_bits`, ties to even. The single terminal rounding of the whole
    /// composite derivation. Returns `None` if the rounded magnitude does not fit `i128`.
    pub fn round_to_scale(&self, scale_bits: u32) -> Option<i128> {
        // scaled = |num| * 2^scale, then divide by den with round-half-to-even.
        let scaled = self.num.shl_bits(scale_bits);
        let (q, r) = scaled.divmod(&self.den);
        // Compare 2*r to den to decide the rounding direction.
        let two_r = r.shl_bits(1);
        let mut q = q;
        match two_r.cmp_big(&self.den) {
            Ordering::Greater => q = q.add(&BigUint::from_u64(1)),
            Ordering::Less => {}
            Ordering::Equal => {
                // Halfway: round to even. Bump only when the current quotient is odd.
                if q.test_bit(0) {
                    q = q.add(&BigUint::from_u64(1));
                }
            }
        }
        let mag = q.to_u128()? as i128;
        Some(if self.neg { -mag } else { mag })
    }

    /// `floor(log2(|self|))` for a non-zero value, used to bracket a composite's magnitude for the
    /// per-quantity scale derivation. Computed by scaling the magnitude up by a large power of two and
    /// reading the bit length, so it is exact integer arithmetic; a rough bracket is all the scale
    /// derivation needs, and the value never approaches the internal `2^-K` floor for a physical constant.
    pub fn floor_log2(&self) -> i64 {
        assert!(!self.num.is_zero(), "floor_log2 of zero");
        const K: u32 = 256;
        let (q, _r) = self.num.shl_bits(K).divmod(&self.den);
        assert!(
            !q.is_zero(),
            "floor_log2 argument underflows the 2^-256 bracket"
        );
        q.bit_len() as i64 - 1 - K as i64
    }

    /// Parse a decimal string (optionally in scientific notation, e.g. `1.380649e-23` or `299792458`)
    /// into an exact rational. Integer arithmetic only, so a datasheet value reaches the rational
    /// losslessly. Errors on malformed input.
    pub fn from_decimal_str(s: &str) -> Result<BigRat, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("empty value".to_string());
        }
        let (mantissa, exp10) = match s.split_once(['e', 'E']) {
            Some((m, e)) => {
                let e: i64 = e
                    .trim()
                    .parse()
                    .map_err(|_| format!("bad exponent in {s}"))?;
                (m, e)
            }
            None => (s, 0i64),
        };
        let (neg, body) = match mantissa.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, mantissa.strip_prefix('+').unwrap_or(mantissa)),
        };
        let (int_part, frac_part) = match body.split_once('.') {
            Some((i, f)) => (i, f),
            None => (body, ""),
        };
        let digits: String = format!("{int_part}{frac_part}");
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("non-numeric mantissa in {s}"));
        }
        // Value = digits * 10^(exp10 - frac_len). Build num/den as powers of ten.
        let num_digits = big_from_dec_digits(&digits)?;
        let net_exp = exp10 - frac_part.len() as i64;
        if net_exp >= 0 {
            Ok(BigRat::new(
                neg,
                num_digits.mul(&BigUint::ten_pow(net_exp as u32)),
                BigUint::from_u64(1),
            ))
        } else {
            Ok(BigRat::new(
                neg,
                num_digits,
                BigUint::ten_pow((-net_exp) as u32),
            ))
        }
    }
}

/// Parse a run of decimal digits into a `BigUint` (Horner over base ten).
fn big_from_dec_digits(digits: &str) -> Result<BigUint, String> {
    let ten = BigUint::from_u64(10);
    let mut acc = BigUint::zero();
    for b in digits.bytes() {
        if !b.is_ascii_digit() {
            return Err(format!("non-digit in {digits}"));
        }
        acc = acc.mul(&ten).add(&BigUint::from_u64((b - b'0') as u64));
    }
    Ok(acc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big(s: &str) -> BigUint {
        big_from_dec_digits(s).unwrap()
    }

    #[test]
    fn add_mul_sub_agree_with_u128_in_range() {
        let cases: [(u64, u64); 6] = [
            (0, 0),
            (1, 1),
            (4_294_967_295, 4_294_967_295),
            (1_000_000_007, 998_244_353),
            (u64::MAX, 3),
            (123_456_789, 987_654_321),
        ];
        for (a, b) in cases {
            let ba = BigUint::from_u64(a);
            let bb = BigUint::from_u64(b);
            assert_eq!(ba.add(&bb).to_u128().unwrap(), a as u128 + b as u128);
            assert_eq!(ba.mul(&bb).to_u128().unwrap(), a as u128 * b as u128);
            let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
            assert_eq!(
                BigUint::from_u64(hi)
                    .sub(&BigUint::from_u64(lo))
                    .to_u128()
                    .unwrap(),
                (hi - lo) as u128
            );
        }
    }

    #[test]
    fn decimal_parse_and_pow_of_ten() {
        assert_eq!(big("0").to_u128().unwrap(), 0);
        assert_eq!(big("299792458").to_u128().unwrap(), 299_792_458);
        assert_eq!(BigUint::ten_pow(0).to_u128().unwrap(), 1);
        assert_eq!(
            BigUint::ten_pow(18).to_u128().unwrap(),
            1_000_000_000_000_000_000
        );
        // 10^29 exceeds u128? 10^29 ~ 1e29 < 3.4e38, fits.
        let d = BigUint::ten_pow(29);
        assert_eq!(d.bit_len(), (1e29_f64).log2().ceil() as u32); // ~97 bits
    }

    #[test]
    fn divmod_reconstructs_dividend() {
        // For many pairs, self == q*divisor + r and r < divisor.
        let dividends = ["0", "1", "1000000", "123456789012345678901234567890"];
        let divisors = ["1", "7", "4294967296", "1000000000000"];
        for ds in dividends {
            for vs in divisors {
                let d = big(ds);
                let v = big(vs);
                let (q, r) = d.divmod(&v);
                assert_eq!(r.cmp_big(&v), Ordering::Less, "{ds}/{vs}: r>=v");
                let recon = q.mul(&v).add(&r);
                assert_eq!(recon, d, "{ds}/{vs}: q*v+r != dividend");
            }
        }
    }

    #[test]
    fn shl_matches_multiply_by_power_of_two() {
        let x = big("123456789012345");
        for bits in [0u32, 1, 31, 32, 33, 64, 100] {
            let shifted = x.shl_bits(bits);
            let by_mul = x.mul(&BigUint::from_u64(2).pow(bits));
            assert_eq!(shifted, by_mul, "shl {bits}");
        }
    }

    #[test]
    fn rational_round_half_to_even_at_scale_zero() {
        // At scale 0, round_to_scale is round-half-to-even of num/den.
        let mk = |n: i64, d: i64| {
            BigRat::new(
                n < 0,
                BigUint::from_u64(n.unsigned_abs()),
                BigUint::from_u64(d as u64),
            )
        };
        assert_eq!(mk(5, 2).round_to_scale(0), Some(2)); // 2.5 -> 2 (even)
        assert_eq!(mk(7, 2).round_to_scale(0), Some(4)); // 3.5 -> 4 (even)
        assert_eq!(mk(1, 3).round_to_scale(0), Some(0)); // 0.333 -> 0
        assert_eq!(mk(2, 3).round_to_scale(0), Some(1)); // 0.666 -> 1
        assert_eq!(mk(-7, 2).round_to_scale(0), Some(-4)); // -3.5 -> -4
    }

    #[test]
    fn rational_arithmetic_signs() {
        let a = BigRat::from_i64(3).div(&BigRat::from_i64(4)); // 3/4
        let b = BigRat::from_i64(1).div(&BigRat::from_i64(2)); // 1/2
        assert_eq!(
            a.sub(&b)
                .cmp_rat(&BigRat::from_i64(1).div(&BigRat::from_i64(4))),
            Ordering::Equal
        ); // 3/4-1/2=1/4
        assert_eq!(b.sub(&a).cmp_rat(&BigRat::from_i64(0)), Ordering::Less); // 1/2-3/4 < 0
        assert_eq!(
            a.mul(&b)
                .cmp_rat(&BigRat::from_i64(3).div(&BigRat::from_i64(8))),
            Ordering::Equal
        ); // 3/8
    }

    #[test]
    fn floor_log2_brackets_magnitudes() {
        // floor(log2(v)) for a few values, including the Stefan-Boltzmann magnitude.
        assert_eq!(BigRat::from_i64(1).floor_log2(), 0); // log2(1) = 0
        assert_eq!(BigRat::from_i64(8).floor_log2(), 3); // log2(8) = 3
        assert_eq!(BigRat::from_i64(255).floor_log2(), 7); // 2^7 = 128 <= 255 < 256
        assert_eq!(BigRat::from_i64(256).floor_log2(), 8);
        // 1/2 -> -1, 1/3 -> -2 (2^-2 = 0.25 <= 1/3 < 0.5)
        assert_eq!(
            BigRat::from_i64(1).div(&BigRat::from_i64(2)).floor_log2(),
            -1
        );
        assert_eq!(
            BigRat::from_i64(1).div(&BigRat::from_i64(3)).floor_log2(),
            -2
        );
        // sigma ~ 5.67e-8, log2 ~ -24.07, floor -25.
        assert_eq!(
            BigRat::from_decimal_str("5.670374419e-8")
                .unwrap()
                .floor_log2(),
            -25
        );
    }

    #[test]
    fn decimal_str_scientific_and_plain() {
        // 1.380649e-23 = 1380649 / 10^29
        let kb = BigRat::from_decimal_str("1.380649e-23").unwrap();
        let expect = BigRat::new(false, big("1380649"), BigUint::ten_pow(29));
        assert_eq!(kb.cmp_rat(&expect), Ordering::Equal);
        // 299792458 = integer
        let c = BigRat::from_decimal_str("299792458").unwrap();
        assert_eq!(c.cmp_rat(&BigRat::from_i64(299_792_458)), Ordering::Equal);
        // negative and positive-exponent forms
        assert_eq!(
            BigRat::from_decimal_str("-2.5e2")
                .unwrap()
                .cmp_rat(&BigRat::from_i64(-250)),
            Ordering::Equal
        );
    }
}
